use std::fs;
use utils::tmpfile::{TmpFile};
use cardano;
use super::Result;

use containers::packfile;
use containers::indexfile;

pub fn create_index(storage: &super::Storage, index: &indexfile::Index) -> (indexfile::Lookup, super::TmpFile) {
    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Index);
    let lookup = index.write_to_tmpfile(&mut tmpfile).unwrap();
    (lookup, tmpfile)
}

pub fn open_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> fs::File {
    fs::File::open(storage_config.get_index_filepath(pack)).unwrap()
}

pub fn dump_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> Result<(indexfile::Lookup, Vec<super::BlockHash>)> {
    let mut file = open_index(storage_config, pack);
    let result = indexfile::dump_file(&mut file)?;
    Ok(result)
}

pub fn read_index_fanout(storage_config: &super::StorageConfig, pack: &super::PackHash) -> Result<indexfile::Lookup> {
    let mut file = open_index(storage_config, pack);
    let lookup = indexfile::Lookup::read_from_file(&mut file)?;
    Ok(lookup)
}

pub fn index_get_header(file: &mut fs::File) -> Result<indexfile::Lookup> {
    let lookup = indexfile::Lookup::read_from_file(file)?;
    Ok(lookup)
}

pub fn packwriter_init(cfg: &super::StorageConfig) -> Result<packfile::Writer> {
    let tmpfile = TmpFile::create(cfg.get_filetype_dir(super::StorageFileType::Pack))?;
    let writer = packfile::Writer::init(tmpfile)?;
    Ok(writer)
}

pub fn packwriter_finalize(cfg: &super::StorageConfig, writer: packfile::Writer) -> (super::PackHash, indexfile::Index) {
    let (tmpfile, packhash, index) = writer.finalize().unwrap();
    let path = cfg.get_pack_filepath(&packhash);
    tmpfile.render_permanent(&path).unwrap();
    (packhash, index)
}

pub fn packreader_init(cfg: &super::StorageConfig, packhash: &super::PackHash) -> packfile::Reader<fs::File> {
    packfile::Reader::init(cfg.get_pack_filepath(packhash)).unwrap()
}

pub fn packreader_block_next(reader: &mut packfile::Reader<fs::File>) -> Option<cardano::block::RawBlock> {
    reader.get_next().and_then(|x| Some(cardano::block::RawBlock(x)))
}
