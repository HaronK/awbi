use crate::bank::Bank;
use crate::file::File;
use anyhow::{bail, ensure, Context, Result};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MemEntryState {
    NotNeeded,
    Loaded,
    LoadMe,
    EndOfMemList,
}

impl MemEntryState {
    pub fn new(state: u8) -> Result<Self> {
        let res = match state {
            0 => MemEntryState::NotNeeded,
            1 => MemEntryState::Loaded,
            2 => MemEntryState::LoadMe,
            0xFF => MemEntryState::EndOfMemList,
            _ => bail!("Unknown entry state {}", state),
        };
        Ok(res)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ResType {
    Sound,
    Music,
    PolyAnim, // full screen video buffer, size=0x7D00

    // FCS: 0x7D00=32000...but 320x200 = 64000 ??
    // Since the game is 16 colors, two pixels palette indices can be stored in one byte
    // that's why we can store two pixels palette index in one byte and we only need 320*200/2 bytes for
    // an entire screen.
    Palette, // palette (1024=vga + 1024=ega), size=2048
    Bytecode,
    PolyCinematic,

    Unknown(u8),
}

impl ResType {
    pub fn new(code: u8) -> Self {
        match code {
            0 => ResType::Sound,
            1 => ResType::Music,
            2 => ResType::PolyAnim,
            3 => ResType::Palette,
            4 => ResType::Bytecode,
            5 => ResType::PolyCinematic,
            _ => ResType::Unknown(code),
        }
    }
}

// This is a directory entry. When the game starts, it loads memlist.bin and
// populate and array of MemEntry
// All resources are packed (for a gain of 28% according to Chahi)
#[derive(Debug)]
pub(crate) struct MemEntry {
    pub state: MemEntryState, // 0x0
    pub res_type: ResType,    // 0x1
    pub buf_offset: usize,    // 0x2
    unk4: u16,                // 0x4, unused
    pub rank_num: u8,         // 0x6
    pub bank_id: u8,          // 0x7
    pub bank_offset: u64,     // 0x8 0xA
    unk_c: u16,               // 0xC, unused
    pub packed_size: usize,   // 0xE
    unk10: u16,               // 0x10, unused
    pub size: usize,          // 0x12
    pub buffer: Vec<u8>,
}

impl MemEntry {
    pub fn from_buf_u8(&self, offset: usize) -> u8 {
        self.buffer[offset]
    }

    pub fn from_buf_be_u16(&self, offset: usize) -> u16 {
        let b1 = self.buffer[offset];
        let b2 = self.buffer[offset + 1];

        u16::from_be_bytes([b1, b2])
    }

    pub fn to_slice(&self, offset: usize, size: usize) -> &[u8] {
        &self.buffer[offset..offset + size]
    }

    pub fn from_slice(&mut self, src: &[u8], offset: usize) {
        self.buffer[offset../*offset + src.len()*/].clone_from_slice(src);
    }

    pub fn read_bank(&self) -> &[u8] {
        &self.buffer
    }
}

#[derive(Debug)]
pub(crate) struct MemList {
    data_dir: String,
    pub entries: Vec<MemEntry>,
}

impl MemList {
    pub fn new(data_dir: &str) -> Self {
        Self {
            data_dir: data_dir.into(),
            entries: Vec::new(),
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let mut f = File::open("memlist.bin", &self.data_dir, false).with_context(|| {
            format!(
                "MemList::load() unable to open '{:?}/memlist.bin' file",
                self.data_dir
            )
        })?;

        loop {
            let entry = MemEntry {
                state: MemEntryState::new(f.read_u8()?)?,
                res_type: ResType::new(f.read_u8()?),
                buf_offset: f.read_u16()? as usize,
                unk4: f.read_u16()?,
                rank_num: f.read_u8()?,
                bank_id: f.read_u8()?,
                bank_offset: f.read_u32()? as u64,
                unk_c: f.read_u16()?,
                packed_size: f.read_u16()? as usize,
                unk10: f.read_u16()?,
                size: f.read_u16()? as usize,
                buffer: Vec::new(),
            };

            if entry.state == MemEntryState::EndOfMemList {
                break;
            }

            self.entries.push(entry);
        }

        Ok(())
    }

    pub fn invalidate_res(&mut self) {
        self.entries
            .iter_mut()
            .filter(|me| me.res_type != ResType::Palette && me.res_type != ResType::Bytecode)
            .for_each(|me| me.state = MemEntryState::NotNeeded);
    }

    pub fn invalidate_all(&mut self) {
        self.entries
            .iter_mut()
            .for_each(|me| me.state = MemEntryState::NotNeeded);
    }
}
