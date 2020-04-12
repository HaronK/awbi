use crate::bank::Bank;
use crate::file::File;
use crate::parts::*;
use crate::reference::*;
use crate::serializer::*;
use anyhow::{bail, ensure, Context, Result};
use std::path::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MemEntryState {
    NotNeeded,
    Loaded,
    LoadMe,
    EndOfMemList,
}

impl MemEntryState {
    fn new(state: u8) -> Result<Self> {
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

// This is a directory entry. When the game starts, it loads memlist.bin and
// populate and array of MemEntry
#[derive(Debug)]
pub(crate) struct MemEntry {
    pub state: MemEntryState, // 0x0
    pub res_type: ResType,    // 0x1
    pub buf_offset: u16,      // 0x2
    unk4: u16,                // 0x4, unused
    rank_num: u8,             // 0x6
    pub bank_id: u8,          // 0x7
    pub bank_offset: u32,     // 0x8 0xA
    unk_c: u16,               // 0xC, unused
    // All resources are packed (for a gain of 28% according to Chahi)
    pub packed_size: u16, // 0xE
    unk10: u16,           // 0x10, unused
    pub size: u16,        // 0x12
    buffer: Vec<u8>,
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
    fn new(code: u8) -> Self {
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

const MEM_BLOCK_SIZE: usize = 600 * 1024; //600kb total memory consumed (not taking into account stack and static heap)

fn read_bank<P: AsRef<Path>>(data_dir: P, me: &MemEntry) -> Result<Vec<u8>> {
    let mut bk = Bank::new(data_dir);
    let res = bk
        .read(me)
        .with_context(|| format!("Resource::readBank() unable to unpack entry"))?;
    ensure!(
        res.len() == me.size as usize,
        "[read_bank] Wrong buffer size. Expected {} but was {}",
        me.size,
        res.len()
    );
    Ok(res)
}

struct ResourceStorage {
    loaded_list: [u8; 64],
    current_part_id: u16,
    script_bak_off: usize,
    script_cur_off: usize,
    vid_bak_off: usize,
    vid_cur_off: usize,
    use_seg_video2: bool,
    seg_palette_idx: usize,
    seg_code_idx: usize,
    seg_cinematic_idx: usize,
    seg_video2_idx: usize,
}

impl Default for ResourceStorage {
    fn default() -> Self {
        Self {
            loaded_list: [0; 64],
            current_part_id: 0,
            script_bak_off: 0,
            script_cur_off: 0,
            vid_bak_off: 0,
            vid_cur_off: 0,
            use_seg_video2: false,
            seg_palette_idx: 0,
            seg_code_idx: 0,
            seg_cinematic_idx: 0,
            seg_video2_idx: 0,
        }
    }
}

// TODO: use proc_macro

impl AccessorWrap for ResourceStorage {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.loaded_list.read(stream)?;
        self.current_part_id.read(stream)?;
        self.script_bak_off.read(stream)?;
        self.script_cur_off.read(stream)?;
        self.vid_bak_off.read(stream)?;
        self.vid_cur_off.read(stream)?;
        self.use_seg_video2.read(stream)?;
        self.seg_palette_idx.read(stream)?;
        self.seg_code_idx.read(stream)?;
        self.seg_cinematic_idx.read(stream)?;
        self.seg_video2_idx.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.loaded_list.write(stream)?;
        self.current_part_id.write(stream)?;
        self.script_bak_off.write(stream)?;
        self.script_cur_off.write(stream)?;
        self.vid_bak_off.write(stream)?;
        self.vid_cur_off.write(stream)?;
        self.use_seg_video2.write(stream)?;
        self.seg_palette_idx.write(stream)?;
        self.seg_code_idx.write(stream)?;
        self.seg_cinematic_idx.write(stream)?;
        self.seg_video2_idx.write(stream)
    }

    fn size(&self) -> usize {
        self.loaded_list.size()
            + self.current_part_id.size()
            + self.script_bak_off.size()
            + self.script_cur_off.size()
            + self.vid_bak_off.size()
            + self.vid_cur_off.size()
            + self.use_seg_video2.size()
            + self.seg_palette_idx.size()
            + self.seg_code_idx.size()
            + self.seg_cinematic_idx.size()
            + self.seg_video2_idx.size()
    }
}

pub(crate) type ResourceRef = Ref<Box<Resource>>;

pub(crate) struct Resource {
    data_dir: PathBuf,
    pub mem_entries: Vec<MemEntry>,
    pub requested_next_part: u16,
    pub mem_buf: [u8; MEM_BLOCK_SIZE],
    storage: ResourceStorage,
}

impl Resource {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf().clone(),
            mem_entries: Vec::new(),
            requested_next_part: 0,
            mem_buf: [0; MEM_BLOCK_SIZE],
            storage: Default::default(),
        }
    }

    pub fn current_part_id(&self) -> u16 {
        self.storage.current_part_id
    }

    pub fn use_seg_video2(&self) -> bool {
        self.storage.use_seg_video2
    }

    pub fn set_use_seg_video2(&mut self, val: bool) {
        self.storage.use_seg_video2 = val;
    }

    pub fn seg_code_idx(&self) -> usize {
        self.storage.seg_code_idx
    }

    pub fn seg_cinematic_idx(&self) -> usize {
        self.storage.seg_cinematic_idx
    }

    pub fn seg_video2_idx(&self) -> usize {
        self.storage.seg_video2_idx
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn memset(&mut self, offset: usize, val: u8, size: usize) {
        for v in &mut self.mem_buf[offset..offset + size] {
            *v = val;
        }
    }

    pub fn mem_to_slice(&self, offset: usize, size: usize) -> &[u8] {
        &self.mem_buf[offset..offset + size]
    }

    pub fn from_mem_be_u16(&self, offset: usize) -> u16 {
        let b1 = self.mem_buf[offset];
        let b2 = self.mem_buf[offset + 1];

        u16::from_be_bytes([b1, b2])
    }

    pub fn from_mem_u8(&self, offset: usize) -> u8 {
        self.mem_buf[offset]
    }

    pub fn read_palette(&self, offset: usize, size: usize) -> &[u8] {
        self.mem_to_slice(self.storage.seg_palette_idx + offset, size)
    }

    // Read all entries from memlist.bin. Do not load anything in memory,
    // this is just a fast way to access the data later based on their id.
    pub fn read_entries(&mut self) -> Result<()> {
        let mut f = File::open("memlist.bin", &self.data_dir, false).with_context(|| {
            format!(
                "Resource::readEntries() unable to open '{:?}/memlist.bin' file",
                self.data_dir
            )
        })?;

        loop {
            let mem_entry = MemEntry {
                state: MemEntryState::new(f.read_u8()?)?,
                res_type: ResType::new(f.read_u8()?),
                buf_offset: f.read_u16()?,
                unk4: f.read_u16()?,
                rank_num: f.read_u8()?,
                bank_id: f.read_u8()?,
                bank_offset: f.read_u32()?,
                unk_c: f.read_u16()?,
                packed_size: f.read_u16()?,
                unk10: f.read_u16()?,
                size: f.read_u16()?,
                buffer: Vec::new(),
            };

            if mem_entry.state == MemEntryState::EndOfMemList {
                break;
            }

            self.mem_entries.push(mem_entry);
        }

        Ok(())
    }

    fn load_marked_as_needed(&mut self) -> Result<()> {
        loop {
            let mut mem_entry: Option<&mut MemEntry> = None;

            // get resource with max rankNum
            let mut max_num = 0;
            for me in &mut self.mem_entries {
                if me.state == MemEntryState::LoadMe && max_num <= me.rank_num {
                    max_num = me.rank_num;
                    mem_entry = Some(me);
                }
            }

            if let Some(me) = &mut mem_entry {
                if me.bank_id == 0 {
                    // warning("Resource::load() ec=0x%X (me->bankId == 0)", 0xF00);
                    me.state = MemEntryState::NotNeeded;
                } else {
                    // debug(DBG_BANK, "Resource::load() bufPos=%X size=%X type=%X pos=%X bankId=%X", loadDestination - _memPtrStart, me->packedSize, me->type, me->bankOffset, me->bankId);
                    let data = read_bank(&self.data_dir, me)?;
                    if me.res_type == ResType::PolyAnim {
                        // video->copyPagePtr(data);
                        me.state = MemEntryState::NotNeeded;
                        todo!(); // TODO:
                    } else {
                        let off = me.buf_offset as usize;
                        // self.mem_buf[off..off + data.len()].copy_from_slice(&data);
                        me.buffer = data;
                        me.state = MemEntryState::Loaded;
                        self.storage.script_cur_off += me.size as usize;
                    }
                }
            } else {
                break; // no entry found
            }
        }

        Ok(())
    }

    pub fn invalidate_res(&mut self) {
        self.mem_entries
            .iter_mut()
            .filter(|me| me.res_type != ResType::Palette && me.res_type != ResType::Bytecode)
            .for_each(|me| me.state = MemEntryState::NotNeeded);
        self.storage.script_cur_off = self.storage.script_bak_off;
    }

    fn invalidate_all(&mut self) {
        self.mem_entries
            .iter_mut()
            .for_each(|me| me.state = MemEntryState::NotNeeded);
        self.storage.script_cur_off = 0;
    }

    pub fn load_parts_or_mem_entry(&mut self, resource_id: u16) -> Result<()> {
        if resource_id as usize > self.mem_entries.len() {
            self.requested_next_part = resource_id;
        } else {
            let mut me = &mut self.mem_entries[resource_id as usize];

            if me.state == MemEntryState::NotNeeded {
                me.state = MemEntryState::LoadMe;
                self.load_marked_as_needed()?;
            }
        }
        Ok(())
    }

    // Protection screen and cinematic don't need the player and enemies polygon data
    // so _memList[video2Index] is never loaded for those parts of the game. When
    // needed (for action phrases) _memList[video2Index] is always loaded with 0x11
    // (as seen in memListParts).
    pub fn setup_part(&mut self, part_id: u16) -> Result<()> {
        if part_id == self.storage.current_part_id {
            return Ok(());
        }

        ensure!(
            part_id >= GAME_PART_FIRST && part_id <= GAME_PART_LAST,
            "Resource::setupPart() ec={} invalid partId",
            part_id
        );

        let part_idx = (part_id - GAME_PART_FIRST) as usize;
        let palette_idx = MEM_LIST_PARTS[part_idx][MEMLIST_PART_PALETTE] as usize;
        let code_idx = MEM_LIST_PARTS[part_idx][MEMLIST_PART_CODE] as usize;
        let video_cinematic_idx = MEM_LIST_PARTS[part_idx][MEMLIST_PART_POLY_CINEMATIC] as usize;
        let video2_idx = MEM_LIST_PARTS[part_idx][MEMLIST_PART_VIDEO2] as usize;

        // Mark all resources as located on hard drive.
        self.invalidate_all();

        self.mem_entries[palette_idx].state = MemEntryState::LoadMe;
        self.mem_entries[code_idx].state = MemEntryState::LoadMe;
        self.mem_entries[video_cinematic_idx].state = MemEntryState::LoadMe;

        // This is probably a cinematic or a non interactive part of the game.
        // Player and enemy polygons are not needed.
        if video2_idx != MEMLIST_PART_NONE {
            self.mem_entries[video2_idx].state = MemEntryState::LoadMe;
        }

        self.load_marked_as_needed()?;

        self.storage.seg_palette_idx = palette_idx;
        self.storage.seg_code_idx = code_idx;
        self.storage.seg_cinematic_idx = video_cinematic_idx;

        // This is probably a cinematic or a non interactive part of the game.
        // Player and enemy polygons are not needed.
        if video2_idx != MEMLIST_PART_NONE {
            self.storage.seg_video2_idx = video2_idx;
        }

        // debug(DBG_RES,"");
        // debug(DBG_RES,"setupPart(%d)",partId-GAME_PART_FIRST);
        // debug(DBG_RES,"Loaded resource %d (%s) in segPalettes.",paletteIndex,resTypeToString(_memList[paletteIndex].type));
        // debug(DBG_RES,"Loaded resource %d (%s) in segBytecode.",codeIndex,resTypeToString(_memList[codeIndex].type));
        // debug(DBG_RES,"Loaded resource %d (%s) in segCinematic.",videoCinematicIndex,resTypeToString(_memList[videoCinematicIndex].type));

        // if video2_idx != MEMLIST_PART_NONE {
        //     debug(DBG_RES,"Loaded resource %d (%s) in _segVideo2.",video2Index,resTypeToString(_memList[video2Index].type));
        // }

        self.storage.current_part_id = part_id;

        // _scriptCurPtr is changed in this->load();
        self.storage.script_bak_off = self.storage.script_cur_off;

        Ok(())
    }

    pub fn reset_mem_block(&mut self) {
        self.mem_buf = [0; MEM_BLOCK_SIZE]; // TODO: faster cleanup?
        self.storage.script_bak_off = 0;
        self.storage.script_cur_off = 0;
        self.storage.vid_bak_off = MEM_BLOCK_SIZE - 0x800 * 16; //0x800 = 2048, so we have 32KB free for vidBack and vidCur
        self.storage.vid_cur_off = self.storage.vid_bak_off;
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        if ser.mode() == Mode::Save {
            let mut ll_idx = 0;
            let mut mem_buf_idx = 0;

            self.storage.loaded_list = [0; 64];

            loop {
                let mut mem_entry = None;

                for (i, me) in self.mem_entries.iter().enumerate() {
                    if me.state == MemEntryState::Loaded && me.buf_offset == mem_buf_idx {
                        mem_entry = Some((i, me));
                        break; // TODO: check this
                    }
                }

                if let Some((i, me)) = mem_entry {
                    self.storage.loaded_list[ll_idx] = i as u8;
                    ll_idx += 1;
                    mem_buf_idx += me.size;
                }
            }
        }

        ser.save_or_load_entries(&mut self.storage, Ver(1))?;

        if ser.mode() == Mode::Load {
            let mut mem_buf_idx = 0;

            for me in &mut self.mem_entries {
                let buf = read_bank(&self.data_dir, me)?;
                me.buf_offset = self.mem_buf.len() as u16;
                // self.mem_buf[mem_buf_idx..mem_buf_idx + buf.len()].copy_from_slice(&buf); // TODO: optimize by reading in read_bank into the slice instead of returning vec
                me.buffer = buf;
                mem_buf_idx += me.size as usize;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> Result<PathBuf> {
        let mut dir = std::env::current_exe()?;

        // Go to project folder
        dir.pop();
        dir.pop();
        dir.pop();
        dir.pop();

        dir.push("data");

        Ok(dir)
    }

    #[test]
    fn test_read_entries() -> Result<()> {
        let data_dir = data_dir()?;
        let mut res = Resource::new(&data_dir);

        res.read_entries()?;

        // println!("Entries:\n{:?}\nLen: {}", res.mem_entries, res.mem_entries.len());

        Ok(())
    }

    #[test]
    fn test_read_bank_packed() -> Result<()> {
        let data_dir = data_dir()?;
        let mut res = Resource::new(&data_dir);

        res.read_entries()?;

        for me in res.mem_entries {
            println!("Entry: {:?}", me);

            test_read_bank(&res.data_dir, &me)?;
        }

        Ok(())
    }

    fn test_read_bank(data_dir: &Path, me: &MemEntry) -> Result<()> {
        let _data = read_bank(data_dir, me)?;
        // println!("Data size: {}", data.len());
        // println!("Data: {:?}", data);

        Ok(())
    }
}
