use rand;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::PathBuf;

pub struct TmpFile {
    file: fs::File,
    path: PathBuf,
}

fn template_create_temp(prefix: &str, suffix: &str) -> String {
    let v1: u64 = rand::random();
    let v2: u64 = rand::random();
    format!("{}{}{}{}", prefix, v1, v2, suffix)
}

impl TmpFile {
    pub fn create(mut path: PathBuf) -> io::Result<Self> {
        let filename = template_create_temp(".tmp.", "");
        path.push(filename);

        OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path)
            .map(|file| TmpFile {
                file: file,
                path: path,
            })
    }

    pub fn render_permanent(&self, path: &PathBuf) -> io::Result<()> {
        // NOTE: we need to consider what is being written, in a case of a tag we want rename
        // to error out correctly in every cases rename fail, however in a case of a hash, since the hash is suppose
        // to represent the same file, some error like EEXIST can be ignored, but some should be raised.
        // NOTE2: also we consider that the rename is atomic for the tmpfile abstraction to work correctly,
        // but it mostly depends on the actual filesystem. POSIX requires it to be atomic.
        match fs::rename(&self.path, path) {
            _ => {}
        };
        Ok(())
    }
}
impl io::Seek for TmpFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}
impl io::Read for TmpFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}
impl io::Write for TmpFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

// write the content buf atomically to the path.
//
// if an issue arise until the data is written, then
// the expected file destination is not going to be
// created
pub fn atomic_write_simple(path: &PathBuf, buf: &[u8]) -> io::Result<()> {
    let mut tmpfile = TmpFile::create(path.parent().unwrap().to_path_buf())?;
    tmpfile.write(buf)?;
    tmpfile.render_permanent(path)?;
    Ok(())
}
