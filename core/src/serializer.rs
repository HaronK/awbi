use crate::file::File;
use anyhow::Result;

#[derive(PartialEq, PartialOrd)]
pub struct Ver(pub u16);

pub const CUR_VER: Ver = Ver(2);

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Save,
    Load,
}

pub trait AccessorWrap {
    fn access(&mut self, mode: Mode, stream: &mut File) -> Result<()> {
        match mode {
            Mode::Save => self.write(stream),
            Mode::Load => self.read(stream),
        }
    }

    fn read(&mut self, stream: &mut File) -> Result<()>;
    fn write(&self, stream: &mut File) -> Result<()>;
    fn size(&self) -> usize;
}

impl AccessorWrap for bool {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        *self = stream.read_u8()? != 0;
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write_u8(*self as u8)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
}

impl AccessorWrap for u8 {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        *self = stream.read_u8()?;
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write_u8(*self)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
}

impl AccessorWrap for u16 {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        *self = stream.read_u16()?;
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write_u16(*self)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u16>()
    }
}

impl AccessorWrap for u32 {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        *self = stream.read_u32()?;
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write_u32(*self)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
}

impl AccessorWrap for usize {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        *self = stream.read_u32()? as usize;
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write_u32(*self as u32)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
}

impl AccessorWrap for [u8] {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        stream.read(self)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write(self)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>() * self.len()
    }
}

impl AccessorWrap for [u16] {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        for i in 0..self.len() {
            self[i] = stream.read_u16()?;
        }
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        for val in self {
            stream.write_u16(*val)?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>() * self.len()
    }
}

impl AccessorWrap for Vec<u8> {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        stream.read(self)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        stream.write(self)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>() * self.len()
    }
}

impl AccessorWrap for Vec<u16> {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        for i in 0..self.len() {
            self[i] = stream.read_u16()?;
        }
        Ok(())
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        for val in self {
            stream.write_u16(*val)?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        std::mem::size_of::<u8>() * self.len()
    }
}

pub struct Serializer {
    stream: File,
    mode: Mode,
    data: Vec<u8>,
    save_ver: Ver,
    bytes_count: u32,
}

impl Serializer {
    pub fn new(stream: File, mode: Mode, data: Vec<u8>, save_ver: Ver) -> Self {
        Self {
            stream,
            mode,
            data,
            save_ver,
            bytes_count: 0,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn save_or_load_entries(
        &mut self,
        accessor: &mut impl AccessorWrap,
        min_ver: Ver,
    ) -> Result<()> {
        // debug(DBG_SER, "Serializer::saveOrLoadEntries() _mode=%d", _mode);
        if self.mode == Mode::Save || self.save_ver >= min_ver && self.save_ver <= CUR_VER {
            accessor.access(self.mode, &mut self.stream)?;
            self.bytes_count = accessor.size() as u32;
        }
        // debug(DBG_SER, "Serializer::saveOrLoadEntries() _bytesCount=%d", _bytesCount);
        Ok(())
    }
}
