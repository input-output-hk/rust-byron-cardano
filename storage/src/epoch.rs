use cardano::block::{BlockDate, EpochId, ChainState};
use cardano::util::hex;
use std::fs;
use std::io::Read;
use chain_state;

use super::{
    header_to_blockhash, packreader_block_next, packreader_init, Error, PackHash, Result, Storage,
    StorageConfig,
};
use storage_units::utils::error::StorageError;
use storage_units::utils::tmpfile;
use storage_units::utils::tmpfile::TmpFile;
use storage_units::{packfile, reffile};

pub fn epoch_create_with_refpack(
    config: &StorageConfig,
    packref: &PackHash,
    refpack: &reffile::Lookup,
    epochid: EpochId,
) {
    let dir = config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    tmpfile::atomic_write_simple(&pack_filepath, hex::encode(packref).as_bytes()).unwrap();

    let mut tmpfile = TmpFile::create(config.get_epoch_dir(epochid)).unwrap();
    refpack.write(&mut tmpfile).unwrap();
    tmpfile
        .render_permanent(&config.get_epoch_refpack_filepath(epochid))
        .unwrap();
}

pub fn epoch_create(
    storage: &Storage,
    packref: &PackHash,
    epochid: EpochId,
    chain_state: Option<&ChainState>,
) {
    // read the pack and append the block hash as we find them in the refpack.
    let mut rp = reffile::Lookup::new();
    let mut reader = packreader_init(&storage.config, packref);

    let mut current_slotid = BlockDate::Boundary(epochid);
    while let Some(rblk) = packreader_block_next(&mut reader) {
        let blk = rblk.decode().unwrap();
        let hdr = blk.get_header();
        let hash = hdr.compute_hash();
        let blockdate = hdr.get_blockdate();

        while current_slotid != blockdate {
            rp.append_missing_hash();
            current_slotid = current_slotid.next();
        }
        rp.append_hash(header_to_blockhash(&hash));
        current_slotid = current_slotid.next();
    }

    let got = reader.finalize();
    assert!(&got == packref);

    // create the directory if not exist
    let dir = storage.config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    // write the refpack
    let mut tmpfile = TmpFile::create(storage.config.get_epoch_dir(epochid)).unwrap();
    rp.write(&mut tmpfile).unwrap();
    tmpfile
        .render_permanent(&storage.config.get_epoch_refpack_filepath(epochid))
        .unwrap();

    // write the utxos
    if let Some(chain_state) = chain_state {
        assert_eq!(chain_state.last_date.unwrap(), BlockDate::Boundary(epochid));
        chain_state::write_chain_state(storage, chain_state).unwrap();
    }

    // write the pack pointer
    let pack_filepath = storage.config.get_epoch_pack_filepath(epochid);
    tmpfile::atomic_write_simple(&pack_filepath, hex::encode(packref).as_bytes()).unwrap();
}

pub fn epoch_read_pack(config: &StorageConfig, epochid: EpochId) -> Result<PackHash> {
    let mut content = Vec::new();

    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    let mut file = fs::File::open(&pack_filepath)?;
    let _read = file.read_to_end(&mut content).unwrap();

    let p = String::from_utf8(content.clone())
        .ok()
        .and_then(|r| hex::decode(&r).ok())
        .unwrap();
    let mut ph = [0u8; super::HASH_SIZE];
    ph.clone_from_slice(&p[..]);

    Ok(ph)
}

pub fn epoch_open_packref(config: &StorageConfig, epochid: EpochId) -> Result<reffile::Reader> {
    let path = config.get_epoch_refpack_filepath(epochid);
    let reader = reffile::Reader::open(path)?;
    Ok(reader)
}

/// Try to open a packfile Reader on a specific epoch
///
/// if there's no pack at this address, then nothing is return
pub fn epoch_open_pack_reader(
    config: &StorageConfig,
    epochid: EpochId,
) -> Result<Option<packfile::Reader<fs::File>>> {
    match epoch_read_pack(config, epochid) {
        Err(Error::StorageError(StorageError::IoError(ref err)))
            if err.kind() == ::std::io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(err) => Err(err),
        Ok(epoch_ref) => {
            let reader = packreader_init(config, &epoch_ref);
            Ok(Some(reader))
        }
    }
}

/*
pub fn epoch_open_pack_seeker() -> io::Result<Option<packfile::Seeker>> {
}
*/

pub fn epoch_read_packref(config: &StorageConfig, epochid: EpochId) -> Result<reffile::Reader> {
    let reader = reffile::Reader::open(config.get_epoch_refpack_filepath(epochid))?;
    Ok(reader)
}

pub fn epoch_read(config: &StorageConfig, epochid: EpochId) -> Result<(PackHash, reffile::Reader)> {
    let ph = epoch_read_pack(config, epochid)?;
    let rp = epoch_read_packref(config, epochid)?;
    Ok((ph, rp))
}

/// Check whether an epoch pack exists on disk.
pub fn epoch_exists(config: &StorageConfig, epochid: EpochId) -> Result<bool> {
    match epoch_read_pack(config, epochid) {
        Ok(_) => Ok(true),
        Err(Error::StorageError(StorageError::IoError(ref err)))
            if err.kind() == ::std::io::ErrorKind::NotFound =>
        {
            Ok(false)
        }
        Err(err) => Err(err),
    }
}
