#[derive(Default)]
pub struct SliceReader {
    pos: usize,
    data: Vec<u8>,
}

impl SliceReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { pos: 0, data }
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
