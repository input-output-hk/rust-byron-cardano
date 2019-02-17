use cardano::block::{BlockDate, ChainState, EpochId};
use cardano::config::GenesisData;
use chain_state;
use std::fs;
use std::io::{Read, Write};

use super::{
    header_to_blockhash, packreader_block_next, packreader_init, Error, PackHash, Result, Storage,
    StorageConfig,
};
use storage_units::utils::error::StorageError;
use storage_units::utils::{tmpfile, serialize};
use storage_units::utils::tmpfile::TmpFile;
use storage_units::{packfile, reffile, indexfile};

pub fn epoch_create_with_refpack(
    config: &StorageConfig,
    packref: &PackHash,
    refpack: &reffile::Lookup,
    epochid: EpochId,
    index: indexfile::Index,
) {
    let dir = config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    epoch_write_pack(config, packref, epochid, index).unwrap();

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
    index: indexfile::Index,
    chain_state: Option<(&ChainState, &GenesisData)>,
) {
    // read the pack and append the block hash as we find them in the refpack.
    let mut rp = reffile::Lookup::new();
    let mut reader = packreader_init(&storage.config, packref);

    let mut current_slotid = BlockDate::Boundary(epochid);
    let mut last_block = None;
    while let Some(rblk) = packreader_block_next(&mut reader).unwrap() {
        let blk = rblk.decode().unwrap();
        let hdr = blk.header();
        let hash = hdr.compute_hash();
        let blockdate = hdr.blockdate();

        while current_slotid != blockdate {
            rp.append_missing_hash();
            current_slotid = current_slotid.next();
        }
        rp.append_hash(header_to_blockhash(&hash));
        current_slotid = current_slotid.next();

        last_block = Some(hash);
    }

    let got = reader.finalize();
    assert!(&got == packref);

    // create the directory if not exist
    let dir = storage.config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    // write the refpack
    {
        let mut tmpfile = TmpFile::create(storage.config.get_epoch_dir(epochid)).unwrap();
        rp.write(&mut tmpfile).unwrap();
        tmpfile
            .render_permanent(&storage.config.get_epoch_refpack_filepath(epochid))
            .unwrap();
    }

    epoch_write_pack(&storage.config, packref, epochid, index).unwrap();

    // write the chain state at the end of the epoch
    // FIXME: should check that chain_state.last_block is actually the
    // last block in the epoch.
    if let Some((chain_state, genesis_data)) = chain_state {
        assert_eq!(chain_state.last_block, last_block.unwrap());
        chain_state::write_chain_state(storage, genesis_data, chain_state).unwrap();
    }
}

// write the pack pointer and ordered block offsets
fn epoch_write_pack(
    storage_cfg: &StorageConfig,
    packref: &PackHash,
    epochid: EpochId,
    index: indexfile::Index,
) -> Result<()> {
    let mut file = tmpfile::TmpFile::create(storage_cfg.get_epoch_dir(epochid)).unwrap();
    // Write fixed size packref hash
    file.write_all(packref)?;
    // Write all ordered offsets
    indexfile::write_offsets_to_file(&mut file, index.offsets.iter())?;
    file
        .render_permanent(&storage_cfg.get_epoch_pack_filepath(epochid))
        .unwrap();
    Ok(())
}

pub fn epoch_read_pack(config: &StorageConfig, epochid: EpochId) -> Result<PackHash> {
    let mut ph = [0u8; super::HASH_SIZE];
    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    let mut file = fs::File::open(&pack_filepath)?;
    file.read_exact(&mut ph)?;
    Ok(ph)
}

pub fn epoch_read_block_offset(config: &StorageConfig, epochid: EpochId, block_index: u32) -> Result<serialize::Offset> {
    let offset_offset = super::HASH_SIZE as u64 + block_index as u64 * serialize::OFF_SIZE as u64;
    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    let file = fs::File::open(&pack_filepath)?;
    Ok(indexfile::file_read_offset_at(&file, offset_offset))
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
