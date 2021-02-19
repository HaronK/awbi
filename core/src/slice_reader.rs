pub struct SliceReader<'a> {
    ip: usize,
    data: &'a [u8],
}

impl<'a> SliceReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { ip: 0, data }
    }

    pub fn can_read(&self) -> bool {
        self.ip < self.data.len()
    }

    /// Get a reference to the slice reader's ip.
    pub fn ip(&self) -> usize {
        self.ip
    }

    // pub fn dec_ip(&mut self, val: usize) {
    //     self.ip -= val;
    // }

    #[inline]
    pub fn read_u8(&mut self) -> u8 {
        let ip = self.ip;
        self.ip += 1;
        self.data[ip]
    }

    #[inline]
    pub fn read_u16(&mut self) -> u16 {
        let ip = self.ip;
        self.ip += 2;
        u16::from_be_bytes([self.data[ip], self.data[ip + 1]])
    }
}
