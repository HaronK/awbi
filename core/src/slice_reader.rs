pub struct SliceReader<'a> {
    addr: usize,
    data: &'a [u8],
}

impl<'a> SliceReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { addr: 0, data }
    }

    pub fn can_read(&self) -> bool {
        self.addr < self.data.len()
    }

    /// Get a reference to the slice reader's ip.
    pub fn addr(&self) -> usize {
        self.addr
    }

    // pub fn dec_addr(&mut self, val: usize) {
    //     self.addr -= val;
    // }

    #[inline]
    pub fn read_u8(&mut self) -> u8 {
        let addr = self.addr;
        self.addr += 1;
        self.data[addr]
    }

    #[inline]
    pub fn read_u16(&mut self) -> u16 {
        let addr = self.addr;
        self.addr += 2;
        u16::from_be_bytes([self.data[addr], self.data[addr + 1]])
    }
}
