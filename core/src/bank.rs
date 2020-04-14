use crate::file::File;
use crate::memlist::MemEntry;
use anyhow::{ensure, Result};
use std::io::{Read, Seek, SeekFrom};

#[derive(Default, Debug)]
pub struct UnpackContext {
    size: u16,
    crc: u32,
    chk: u32,
    data_size: u32,
}

/// Packed data. Access values in reverse order.
#[derive(Default)]
struct PackedData {
    data: Vec<u8>,
    pos: usize,
}

impl PackedData {
    fn new(data: Vec<u8>) -> Self {
        let pos = data.len();
        Self { data, pos }
    }

    fn read(&mut self) -> u32 {
        self.pos -= 4;
        u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ])
    }
}

/// Packed data. Access values in reverse order.
#[derive(Default)]
struct UnpackedData {
    data: Vec<u8>,
    pos: usize,
}

impl UnpackedData {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
            pos: size,
        }
    }

    fn write(&mut self, b: u8) {
        self.pos -= 1;
        self.data[self.pos] = b;
    }
}

#[derive(Default)]
pub(crate) struct Bank {
    unp_ctx: UnpackContext,
    packed: PackedData,
    unpacked: UnpackedData,
}

impl Bank {
    pub fn read(&mut self, data_dir: &str, me: &MemEntry) -> Result<Vec<u8>> {
        let mut f = self.read_bank(data_dir, me.bank_id)?;

        self.read_entry_data(&mut f.file_impl, me)
    }

    pub fn read_bank(&self, data_dir: &str, bank_id: u8) -> Result<File> {
        let bank_name = format!("bank{:02x}", bank_id);
        File::open(&bank_name, data_dir, false)
    }

    pub fn read_entry_data<T: Read + Seek>(
        &mut self,
        src: &mut T,
        me: &MemEntry,
    ) -> Result<Vec<u8>> {
        let mut buf = vec![0; me.packed_size];

        src.seek(SeekFrom::Start(me.bank_offset))?;
        src.read_exact(&mut buf)?;

        // Depending if the resource is packed or not we
        // can read directly or unpack it.

        if me.packed_size == me.size {
            Ok(buf)
        } else {
            self.packed = PackedData::new(buf);
            self.unpack()
        }
    }

    fn unpack(&mut self) -> Result<Vec<u8>> {
        self.unp_ctx.size = 0;
        self.unp_ctx.data_size = self.packed.read();
        self.unp_ctx.crc = self.packed.read();
        self.unp_ctx.chk = self.packed.read();
        self.unp_ctx.crc ^= self.unp_ctx.chk;

        self.unpacked = UnpackedData::new(self.unp_ctx.data_size as usize);

        while self.unp_ctx.data_size > 0 {
            if !self.next_chunk() {
                self.unp_ctx.size = 1;
                if !self.next_chunk() {
                    self.dec_unk1(3, 0);
                } else {
                    self.dec_unk2(8);
                }
            } else {
                let c = self.get_code(2);
                if c == 3 {
                    self.dec_unk1(8, 8);
                } else if c < 2 {
                    self.unp_ctx.size = c + 2;
                    self.dec_unk2(c as u8 + 9);
                } else {
                    self.unp_ctx.size = self.get_code(8);
                    self.dec_unk2(12);
                }
            }
        }
        ensure!(self.unp_ctx.crc == 0, "CRC should be 0");

        Ok(self.unpacked.data.clone())
    }

    fn dec_unk1(&mut self, num_chunks: u8, add_count: u8) {
        let count = self.get_code(num_chunks) + add_count as u16 + 1;
        // debug(DBG_BANK, "Bank::decUnk1(%d, %d) count=%d", numChunks, addCount, count);
        self.unp_ctx.data_size -= count as u32;
        for _ in 0..count {
            let val = self.get_code(8) as u8;
            self.unpacked.write(val);
        }
    }

    /*
       Note from fab: This look like run-length encoding.
    */
    fn dec_unk2(&mut self, num_chunks: u8) {
        let i = self.get_code(num_chunks) as usize;
        let count = self.unp_ctx.size + 1;
        self.unp_ctx.data_size -= count as u32;

        // println!("dec_unk2({}): i={} count={} unp_pos={} size={}",
        //     num_chunks, i, count, self.unpacked.pos, self.unpacked.data.len());

        for _ in 0..count {
            let val = self.unpacked.data[self.unpacked.pos + i - 1];
            self.unpacked.write(val);
        }
    }

    fn get_code(&mut self, num_chunks: u8) -> u16 {
        let mut c = 0;
        for _ in 0..num_chunks {
            c <<= 1;
            if self.next_chunk() {
                c |= 1;
            }
        }
        c
    }

    fn next_chunk(&mut self) -> bool {
        let mut cf = self.rcr(false);
        if self.unp_ctx.chk == 0 {
            self.unp_ctx.chk = self.packed.read();
            self.unp_ctx.crc ^= self.unp_ctx.chk;
            cf = self.rcr(true);
        }
        cf
    }

    fn rcr(&mut self, cf: bool) -> bool {
        let rcf = (self.unp_ctx.chk & 1) != 0;
        self.unp_ctx.chk >>= 1;
        if cf {
            self.unp_ctx.chk |= 0x8000_0000;
        }
        rcf
    }
}
