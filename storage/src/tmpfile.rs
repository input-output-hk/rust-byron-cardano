use rand;
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct TmpFile {
    file: fs::File,
    path: PathBuf,
}
impl TmpFile {
    pub fn create(mut path: PathBuf) -> io::Result<Self> {
        let v1 : u64 = rand::random();
        let v2 : u64 = rand::random();
        path.push(format!(".tmp.{}{}", v1, v2));

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map(|file| TmpFile { file: file, path: path })
    }

    pub fn render_permanent(&self, path: &PathBuf) -> io::Result<()> {
        // NOTE: we need to consider what is being written, in a case of a tag we want rename
        // to error out correctly in every cases rename fail, however in a case of a hash, since the hash is suppose
        // to represent the same file, some error like EEXIST can be ignored, but some should be raised.
        // NOTE2: also we consider that the rename is atomic for the tmpfile abstraction to work correctly,
        // but it mostly depends on the actual filesystem. for most case it should be atomic.
        match fs::rename(&self.path, path) {
            _ => {},
        };
        Ok(())
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
    fn flush(&mut self) -> io::Result<()> { self.file.flush() }
}
