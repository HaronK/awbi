// TODO: creeate a trait and Vec and slice implementations

#[derive(Clone, Default)]
pub struct SliceReader {
    pos: usize,
    data: Vec<u8>,
}

impl SliceReader {
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn can_read(&self) -> bool {
        self.pos < self.data.len()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self.data[start..end]
    }

    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn read_u8(&mut self) -> u8 {
        let addr = self.pos;
        self.pos += 1;
        self.data[addr]
    }

    #[inline]
    pub fn read_u16(&mut self) -> u16 {
        let addr = self.pos;
        self.pos += 2;
        u16::from_be_bytes([self.data[addr], self.data[addr + 1]])
    }
}

impl From<Vec<u8>> for SliceReader {
    fn from(data: Vec<u8>) -> Self {
        Self { pos: 0, data }
    }
}

impl From<&[u8]> for SliceReader {
    fn from(data: &[u8]) -> Self {
        Self {
            pos: 0,
            data: data.into(),
        }
    }
}
