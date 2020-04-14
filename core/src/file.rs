use anyhow::{bail, Result};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::*;

pub(crate) trait FileImpl: std::fmt::Debug + Read + Seek + Write {}

#[derive(Debug)]
pub(crate) struct File {
    pub file_impl: Box<dyn FileImpl>,
}

impl File {
    pub fn open<P: AsRef<Path>>(filename: &str, directory: P, zipped: bool) -> Result<Self> {
        let mut path = directory.as_ref().to_path_buf();
        path.push(filename);

        let file_impl: Box<dyn FileImpl> = if zipped {
            Box::new(ZipFile::open(path)?)
        } else {
            Box::new(StdFile::open(&path)?)
        };
        Ok(Self { file_impl })
    }

    pub fn seek(&mut self, off: u64) -> Result<()> {
        self.file_impl.seek(SeekFrom::Start(off))?;
        Ok(())
    }

    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        self.file_impl.read_to_end(&mut data)?;
        Ok(data)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<()> {
        self.file_impl.read_exact(buf)?;
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let hi = self.read_u8()?;
        let mut res = self.read_u8()? as u16;
        res |= (hi as u16) << 8;
        Ok(res)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let hi = self.read_u16()?;
        let mut res = self.read_u16()? as u32;
        res |= (hi as u32) << 16;
        Ok(res)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.file_impl.write_all(buf)?;
        Ok(())
    }

    pub fn write_be(&mut self, buf: &[u8]) -> Result<()> {
        match buf.len() {
            1 => self.write_u8(buf[0])?,
            2 => self.write_u16(u16::from_ne_bytes([buf[0], buf[1]]))?,
            4 => self.write_u32(u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]))?,
            _ => bail!("[save_int] Unsupported size: {}", buf.len()),
        }
        Ok(())
    }

    pub fn write_u8(&mut self, b: u8) -> Result<()> {
        let buf = [b];
        self.write(&buf)
    }

    pub fn write_u16(&mut self, n: u16) -> Result<()> {
        self.write_u8((n >> 8) as u8)?;
        self.write_u8((n & 0xFF) as u8)
    }

    pub fn write_u32(&mut self, n: u32) -> Result<()> {
        self.write_u16((n >> 16) as u16)?;
        self.write_u16((n & 0xFFFF) as u16)
    }
}

#[derive(Debug)]
struct StdFile {
    file: std::fs::File,
}

impl StdFile {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            file: std::fs::File::open(path.as_ref())?,
        })
    }
}

impl FileImpl for StdFile {}

impl Seek for StdFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Read for StdFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for StdFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[derive(Debug)]
struct ZipFile {
    file: std::fs::File,
}

impl ZipFile {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            file: std::fs::File::open(path.as_ref())?,
        })
    }
}

impl FileImpl for ZipFile {}

impl Seek for ZipFile {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        todo!();
    }
}

impl Read for ZipFile {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        todo!();
    }
}

impl Write for ZipFile {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        todo!();
    }
    fn flush(&mut self) -> std::io::Result<()> {
        todo!();
    }
}
