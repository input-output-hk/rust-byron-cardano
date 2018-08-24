use std::fs;
use std::io;
use std::io::{Read};
use cardano::util::{hex};

use cardano;

use super::{StorageConfig, PackHash, packreader_init, packreader_block_next, header_to_blockhash};
use super::utils::tmpfile;
use super::utils::tmpfile::{TmpFile};
use super::containers::{packfile, reffile};

pub fn epoch_create_with_refpack(config: &StorageConfig, packref: &PackHash, refpack: &reffile::Lookup, epochid: cardano::block::EpochId) {
    let dir = config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    tmpfile::atomic_write_simple(&pack_filepath, hex::encode(packref).as_bytes()).unwrap();

    let mut tmpfile = TmpFile::create(config.get_epoch_dir(epochid)).unwrap();
    refpack.write(&mut tmpfile).unwrap();
    tmpfile.render_permanent(&config.get_epoch_refpack_filepath(epochid)).unwrap();
}

pub fn epoch_create(config: &StorageConfig, packref: &PackHash, epochid: cardano::block::EpochId) {
    // read the pack and append the block hash as we find them in the refpack.
    let mut rp = reffile::Lookup::new();
    let mut reader = packreader_init(config, packref);

    let mut current_slotid = cardano::block::BlockDate::Genesis(epochid);
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
    let dir = config.get_epoch_dir(epochid);
    fs::create_dir_all(dir).unwrap();

    // write the refpack
    let mut tmpfile = TmpFile::create(config.get_epoch_dir(epochid)).unwrap();
    rp.write(&mut tmpfile).unwrap();
    tmpfile.render_permanent(&config.get_epoch_refpack_filepath(epochid)).unwrap();

    // write the pack pointer
    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    tmpfile::atomic_write_simple(&pack_filepath, hex::encode(packref).as_bytes()).unwrap();
}

pub fn epoch_read_pack(config: &StorageConfig, epochid: cardano::block::EpochId) -> io::Result<PackHash> {
    let mut content = Vec::new();

    let pack_filepath = config.get_epoch_pack_filepath(epochid);
    let mut file = fs::File::open(&pack_filepath)?;
    let _read = file.read_to_end(&mut content).unwrap();

    let p = String::from_utf8(content.clone()).ok().and_then(|r| hex::decode(&r).ok()).unwrap();
    let mut ph = [0u8; super::HASH_SIZE];
    ph.clone_from_slice(&p[..]);

    Ok(ph)
}

pub fn epoch_open_packref(config: &StorageConfig, epochid: cardano::block::EpochId) -> io::Result<reffile::Reader> {
    let path = config.get_epoch_refpack_filepath(epochid);
    reffile::Reader::open(path)
}

/// Try to open a packfile Reader on a specific epoch
///
/// if there's no pack at this address, then nothing is return
pub fn epoch_open_pack_reader(config: &StorageConfig, epochid: cardano::block::EpochId) -> io::Result<Option<packfile::Reader<fs::File>>> {
    match epoch_read_pack(config, epochid) {
        Err(err) => {
            if err.kind() == ::std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(err)
            }
        },
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

pub fn epoch_read_packref(config: &StorageConfig, epochid: cardano::block::EpochId) -> io::Result<reffile::Reader> {
    reffile::Reader::open(config.get_epoch_refpack_filepath(epochid))
}

pub fn epoch_read(config: &StorageConfig, epochid: cardano::block::EpochId) -> io::Result<(PackHash, reffile::Reader)> {
    match epoch_read_pack(config, epochid) {
        Err(e) => Err(e),
        Ok(ph) => {
            let rp = epoch_read_packref(config, epochid)?;
            Ok((ph, rp))
        }
    }
}
