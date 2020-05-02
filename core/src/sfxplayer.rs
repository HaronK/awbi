use crate::file::*;
use crate::memlist::*;
use crate::mixer::*;
use crate::reference::*;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use anyhow::{bail, Result};

#[derive(Clone, Copy, Default)]
struct SfxInstrument {
    buf_offset: u16,
    volume: u16,
}

struct SfxModule {
    buf_offset: u16,
    cur_pos: u16,
    cur_order: u8,
    num_order: u8,
    order_table: [u8; 0x80],
    samples: [SfxInstrument; 15],
}

impl Default for SfxModule {
    fn default() -> Self {
        Self {
            buf_offset: 0,
            cur_pos: 0,
            cur_order: 0,
            num_order: 0,
            order_table: [0; 0x80],
            samples: [Default::default(); 15],
        }
    }
}

// TODO: use proc_macro

impl AccessorWrap for SfxModule {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.cur_pos.read(stream)?;
        self.cur_order.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.cur_pos.write(stream)?;
        self.cur_order.write(stream)
    }

    fn size(&self) -> usize {
        self.cur_pos.size() + self.cur_order.size()
    }
}

struct SfxPattern {
    note_1: u16,
    note_2: u16,
    sample_start: u16,
    sample_buffer: Vec<u8>,
    sample_len: u16,
    loop_pos: u16,
    loop_data: Vec<u8>,
    loop_len: u16,
    sample_volume: u16,
}

pub(crate) type SfxPlayerRef = Ref<Box<SfxPlayer>>;

pub(crate) struct SfxPlayer {
    mixer: MixerRef,
    res: ResourceRef,
    sys: SystemRef,

    mutex: Vec<u8>,
    timer_id: Vec<u8>,
    delay: u16,
    res_id: u16,
    sfx_mod: SfxModule,
    pub mark_var: Vec<i16>,
}

impl SfxPlayer {
    pub fn new(mixer: MixerRef, res: ResourceRef, sys: SystemRef) -> Self {
        Self {
            mixer,
            res,
            sys,
            mutex: Vec::new(),
            timer_id: Vec::new(),
            delay: 0,
            res_id: 0,
            sfx_mod: Default::default(),
            mark_var: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        self.mutex = self.sys.get_mut().create_mutex();
    }

    fn free(&mut self) {
        self.stop();
        self.sys.get_mut().destroy_mutex(&self.mutex);
    }

    pub fn set_events_delay(&mut self, delay: u16) {
        // debug(DBG_SND, "SfxPlayer::setEventsDelay(%d)", delay);
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.delay = delay * 60 / 7050;
    }

    pub fn load_sfx_module(&mut self, res_id: u16, delay: u16, pos: u8) -> Result<()> {
        // debug(DBG_SND, "SfxPlayer::loadSfxModule(0x%X, %d, %d)", resNum, delay, pos);
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);

        // to avoid borrow checker complain
        let me = &self.res.get().storage.mem_list.entries[res_id as usize];
        let me_offset = me.buf_offset as usize;

        if me.state == MemEntryState::Loaded && me.res_type == ResType::Music {
            self.res_id = res_id;
            self.sfx_mod = Default::default();
            self.sfx_mod.cur_order = pos;
            self.sfx_mod.num_order = me.from_buf_be_u16(me_offset + 0x3E) as u8;
            // debug(DBG_SND, "SfxPlayer::loadSfxModule() curOrder = 0x%X numOrder = 0x%X", _sfxMod.curOrder, _sfxMod.numOrder);

            // for i in 0..0x80 {
            //     self.sfx_mod.order_table[i] = me.from_buf_u8(me_offset + 0x40 + i);
            // }
            self.sfx_mod.order_table[..].clone_from_slice(me.to_slice(me_offset + 0x40, 0x80));

            if delay == 0 {
                self.delay = me.from_buf_be_u16(me_offset);
            } else {
                self.delay = delay;
            }
            self.delay *= 60 / 7050;
            self.sfx_mod.buf_offset = (me_offset as u16) + 0xC0;
        // debug(DBG_SND, "SfxPlayer::loadSfxModule() eventDelay = %d ms", _delay);

        // self.prepare_instruments(res, &me, me_offset + 2)?;
        } else {
            //     warning("SfxPlayer::loadSfxModule() ec=0x%X", 0xF8);
        }

        Ok(())
    }

    fn prepare_instruments(
        &mut self,
        res: &mut Resource,
        src_me: &MemEntry,
        mut offset: usize,
    ) -> Result<()> {
        for ins in &mut self.sfx_mod.samples {
            let res_id = src_me.from_buf_be_u16(offset as usize) as usize;
            offset += 2;

            if res_id != 0 {
                ins.volume = src_me.from_buf_be_u16(offset as usize);
                let me = &res.storage.mem_list.entries[res_id];

                if me.state == MemEntryState::Loaded && me.res_type == ResType::Sound {
                    ins.buf_offset = me.buf_offset as u16;
                    res.memset(ins.buf_offset as usize + 8, 0, 4);
                //         debug(DBG_SND, "Loaded instrument 0x%X n=%d volume=%d", resNum, i, ins->volume);
                } else {
                    bail!("Error loading instrument {}", res_id);
                }
            }

            offset += 2; // skip volume
        }

        Ok(())
    }

    pub fn start(&mut self) {
        // debug(DBG_SND, "SfxPlayer::start()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.sfx_mod.cur_pos = 0;
        // self.timer_id = self.sys.get_mut().add_timer(self.delay as u32, &|_interval| { self.handle_events(); self.delay as u32 }); // TODO: uncomment
    }

    pub fn stop(&mut self) {
        // debug(DBG_SND, "SfxPlayer::stop()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        if self.res_id != 0 {
            self.res_id = 0;
            self.sys.get_mut().remove_timer(&self.timer_id);
        }
    }

    fn handle_events(&mut self) {
        todo!(); // TODO: implement
    }

    fn handle_pattern(&mut self, _channel: u8, _pattern_data: &[u8]) {
        todo!(); // TODO: implement
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        self.sys.get_mut().lock_mutex(&self.mutex);

        ser.save_or_load_entries(self, Ver(2))?;

        self.sys.get_mut().unlock_mutex(&self.mutex);

        if ser.mode() == Mode::Load && self.res_id != 0 {
            let delay = self.delay;
            self.load_sfx_module(self.res_id, 0, self.sfx_mod.cur_order)?;
            self.delay = delay;
            // self.timer_id = self.sys.get_mut().add_timer(self.delay as u32, &|_interval| { self.handle_events(); self.delay as u32 }); // TODO: uncomment
        }

        Ok(())
    }
}

// TODO: use proc_macro

impl AccessorWrap for SfxPlayer {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.delay.read(stream)?;
        self.res_id.read(stream)?;
        self.sfx_mod.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.delay.write(stream)?;
        self.res_id.write(stream)?;
        self.sfx_mod.write(stream)
    }

    fn size(&self) -> usize {
        self.delay.size() + self.res_id.size() + self.sfx_mod.size()
    }
}
