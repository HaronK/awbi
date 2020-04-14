use crate::file::File;
use crate::memlist::*;
use crate::parts::*;
use crate::reference::*;
use crate::serializer::*;
use anyhow::{ensure, Result};

use trace::trace;

trace::init_depth_var!();

const MEM_BLOCK_SIZE: usize = 600 * 1024; //600kb total memory consumed (not taking into account stack and static heap)

struct ResourceData {
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

impl Default for ResourceData {
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

impl std::fmt::Debug for ResourceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceStorage")
            .field("loaded_list.size", &self.loaded_list.len())
            // .field("loaded_list", &self.loaded_list[..32].iter().collect::<Vec<_>>())
            .field("current_part_id", &self.current_part_id)
            .field("script_bak_off", &self.script_bak_off)
            .field("script_cur_off", &self.script_cur_off)
            .field("vid_bak_off", &self.vid_bak_off)
            .field("vid_cur_off", &self.vid_cur_off)
            .field("use_seg_video2", &self.use_seg_video2)
            .field("seg_palette_idx", &self.seg_palette_idx)
            .field("seg_code_idx", &self.seg_code_idx)
            .field("seg_cinematic_idx", &self.seg_cinematic_idx)
            .field("seg_video2_idx", &self.seg_video2_idx)
            .finish()
    }
}

// TODO: use proc_macro

impl AccessorWrap for ResourceData {
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
    pub mem_list: MemList,
    pub requested_next_part: u16,
    pub mem_buf: [u8; MEM_BLOCK_SIZE],
    data: ResourceData,
}

impl Resource {
    pub fn new(mem_list: MemList) -> Self {
        Self {
            mem_list,
            requested_next_part: 0,
            mem_buf: [0; MEM_BLOCK_SIZE],
            data: Default::default(),
        }
    }

    pub fn current_part_id(&self) -> u16 {
        self.data.current_part_id
    }

    pub fn use_seg_video2(&self) -> bool {
        self.data.use_seg_video2
    }

    pub fn set_use_seg_video2(&mut self, val: bool) {
        self.data.use_seg_video2 = val;
    }

    pub fn seg_code_idx(&self) -> usize {
        self.data.seg_code_idx
    }

    pub fn seg_cinematic_idx(&self) -> usize {
        self.data.seg_cinematic_idx
    }

    pub fn seg_video2_idx(&self) -> usize {
        self.data.seg_video2_idx
    }

    // pub fn data_dir(&self) -> &String {
    //     &self.data_dir
    // }

    pub fn memset(&mut self, offset: usize, val: u8, size: usize) {
        for v in &mut self.mem_buf[offset..offset + size] {
            *v = val;
        }
    }

    pub fn mem_to_slice(&self, offset: usize, size: usize) -> &[u8] {
        &self.mem_buf[offset..offset + size]
    }

    // #[trace]
    pub fn from_mem_u8(&self, _page_idx: usize, offset: usize) -> u8 {
        // self.mem_entries[page_idx].from_buf_u8(offset)
        self.mem_buf[offset]
    }

    // #[trace]
    pub fn from_mem_be_u16(&self, _page_idx: usize, offset: usize) -> u16 {
        // self.mem_entries[page_idx].from_buf_be_u16(offset)
        let b1 = self.mem_buf[offset];
        let b2 = self.mem_buf[offset + 1];

        u16::from_be_bytes([b1, b2])
    }

    // #[trace]
    pub fn read_palette(&self, offset: usize, size: usize) -> &[u8] {
        self.mem_to_slice(self.data.seg_palette_idx + offset, size)
    }

    // Read all entries from memlist.bin. Do not load anything in memory,
    // this is just a fast way to access the data later based on their id.
    #[trace]
    pub fn read_entries(&mut self) -> Result<()> {
        self.mem_list.load()
    }

    #[trace]
    fn load_marked_as_needed(&mut self) -> Result<()> {
        loop {
            let mut mem_entry: Option<&mut MemEntry> = None;

            // get resource with max rankNum
            let mut max_num = 0;

            for me in &mut self.mem_list.entries {
                if me.state == MemEntryState::LoadMe && max_num <= me.rank_num {
                    max_num = me.rank_num;
                    mem_entry = Some(me);
                }
            }

            if let Some(me) = &mut mem_entry {
                // At this point the resource descriptor should be pointed to "me"
                // "That's what she said"

                if me.bank_id == 0 {
                    // warning("Resource::load() ec=0x%X (me->bankId == 0)", 0xF00);
                    me.state = MemEntryState::NotNeeded;
                } else {
                    // debug(DBG_BANK, "Resource::load() bufPos=%X size=%X type=%X pos=%X bankId=%X", loadDestination - _memPtrStart, me->packedSize, me->type, me->bankOffset, me->bankId);
                    let data = me.read_bank("me")?; // TODO: fix 'me'
                    if me.res_type == ResType::PolyAnim {
                        // self.mem_entries[self.storage.seg_video2_idx]
                        //     .from_slice(&data, self.storage.vid_cur_off);
                        // self.video.copy_page_data(&data); // TODO: uncomment
                        me.state = MemEntryState::NotNeeded;
                        todo!(); // TODO:
                    } else {
                        if me.size as usize > self.data.vid_bak_off - self.data.script_cur_off {
                            // warning("Resource::load() not enough memory");
                            me.state = MemEntryState::NotNeeded;
                            continue;
                        }
                        // self.mem_entries[self.storage.seg_code_idx]
                        //     .from_slice(&data, self.storage.script_cur_off);
                        let off = me.buf_offset as usize;
                        self.mem_buf[off..off + data.len()].copy_from_slice(&data);
                        me.buffer = data;
                        me.state = MemEntryState::Loaded;
                        self.data.script_cur_off += me.size as usize;
                    }
                }
            } else {
                break; // no entry found
            }
        }

        Ok(())
    }

    #[trace]
    pub fn invalidate_res(&mut self) {
        self.mem_list.invalidate_res();
        self.data.script_cur_off = self.data.script_bak_off;
    }

    #[trace]
    fn invalidate_all(&mut self) {
        self.mem_list.invalidate_all();
        self.data.script_cur_off = 0;
    }

    #[trace]
    pub fn load_parts_or_mem_entry(&mut self, resource_id: u16) -> Result<()> {
        if resource_id as usize > self.mem_list.entries.len() {
            self.requested_next_part = resource_id;
        } else {
            let mut me = &mut self.mem_list.entries[resource_id as usize];

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
    #[trace]
    pub fn setup_part(&mut self, part_id: u16) -> Result<()> {
        if part_id == self.data.current_part_id {
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

        self.mem_list.entries[palette_idx].state = MemEntryState::LoadMe;
        self.mem_list.entries[code_idx].state = MemEntryState::LoadMe;
        self.mem_list.entries[video_cinematic_idx].state = MemEntryState::LoadMe;

        // This is probably a cinematic or a non interactive part of the game.
        // Player and enemy polygons are not needed.
        if video2_idx != MEMLIST_PART_NONE {
            self.mem_list.entries[video2_idx].state = MemEntryState::LoadMe;
        }

        self.load_marked_as_needed()?;

        self.data.seg_palette_idx = palette_idx;
        self.data.seg_code_idx = code_idx;
        self.data.seg_cinematic_idx = video_cinematic_idx;

        // This is probably a cinematic or a non interactive part of the game.
        // Player and enemy polygons are not needed.
        if video2_idx != MEMLIST_PART_NONE {
            self.data.seg_video2_idx = video2_idx;
        }

        println!("\tpart_idx={}", part_idx);
        println!(
            "\tpalette_idx={} {:?}",
            palette_idx, self.mem_list.entries[palette_idx].res_type
        );
        println!(
            "\tcode_idx={} {:?}",
            code_idx, self.mem_list.entries[code_idx].res_type
        );
        println!(
            "\tvideo_cinematic_idx={} {:?}",
            video_cinematic_idx, self.mem_list.entries[video_cinematic_idx].res_type
        );

        if video2_idx != MEMLIST_PART_NONE {
            println!(
                "\tvideo2_idx={} {:?}",
                video2_idx, self.mem_list.entries[video2_idx].res_type
            );
        }

        self.data.current_part_id = part_id;

        // _scriptCurPtr is changed in this->load();
        self.data.script_bak_off = self.data.script_cur_off;

        Ok(())
    }

    #[trace]
    pub fn reset_mem_block(&mut self) {
        self.mem_buf = [0; MEM_BLOCK_SIZE]; // TODO: faster cleanup?
        self.data.script_bak_off = 0;
        self.data.script_cur_off = 0;
        self.data.vid_bak_off = MEM_BLOCK_SIZE - 0x800 * 16; //0x800 = 2048, so we have 32KB free for vidBack and vidCur
        self.data.vid_cur_off = self.data.vid_bak_off;
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        if ser.mode() == Mode::Save {
            let mut ll_idx = 0;
            let mut mem_buf_idx = 0;

            self.data.loaded_list = [0; 64];

            loop {
                let mut mem_entry = None;

                for (i, me) in self.mem_list.entries.iter().enumerate() {
                    if me.state == MemEntryState::Loaded && me.buf_offset == mem_buf_idx {
                        mem_entry = Some((i, me));
                        break; // TODO: check this
                    }
                }

                if let Some((i, me)) = mem_entry {
                    self.data.loaded_list[ll_idx] = i as u8;
                    ll_idx += 1;
                    mem_buf_idx += me.size;
                } else {
                    break;
                }
            }
        }

        ser.save_or_load_entries(&mut self.data, Ver(1))?;

        if ser.mode() == Mode::Load {
            let mut mem_buf_idx = 0;

            for me in &mut self.mem_list.entries {
                let buf = me.read_bank("me")?; // TODO: fix'me'
                me.buf_offset = self.mem_buf.len();
                self.mem_buf[mem_buf_idx..mem_buf_idx + buf.len()].copy_from_slice(&buf); // TODO: optimize by reading in read_bank into the slice instead of returning vec
                me.buffer = buf;
                mem_buf_idx += me.size as usize;
                todo!();
            }
        }

        Ok(())
    }
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("data_dir", &self.mem_list)
            .field("requested_next_part", &self.requested_next_part)
            .field("mem_buf.size", &self.mem_buf.len())
            //  .field("mem_buf", &self.mem_buf[..32].iter().collect::<Vec<_>>())
            .field("data", &self.data)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::*;

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
    #[ignore]
    fn test_read_entries() -> Result<()> {
        let data_dir = data_dir()?;
        let mem_list = MemList::new(data_dir.to_str().unwrap());
        let mut res = Resource::new(mem_list);

        res.read_entries()?;

        // println!("Entries:\n{:?}\nLen: {}", res.mem_entries, res.mem_entries.len());

        Ok(())
    }

    #[test]
    // #[ignore]
    fn test_read_all_banks() -> Result<()> {
        let data_dir: String = data_dir()?.to_str().unwrap().into();
        let mem_list = MemList::new(&data_dir);
        let mut res = Resource::new(mem_list);

        res.read_entries()?;

        for me in res.mem_list.entries {
            println!("Entry: {:?}", me);

            test_read_bank(&data_dir, &me)?;
        }

        Ok(())
    }

    fn test_read_bank(data_dir: &str, me: &MemEntry) -> Result<()> {
        let _data = me.read_bank(data_dir)?;
        // println!("Data size: {}", data.len());
        // println!("Data: {:?}", data);

        Ok(())
    }
}
