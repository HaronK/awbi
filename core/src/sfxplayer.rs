use crate::file::*;
use crate::mixer::*;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use crate::reference::*;
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

pub type SfxPlayerRef = Ref<Box<SfxPlayer>>;

struct SfxPlayer {
	mixer: MixerRef,
	res: ResourceRef,
	sys: SystemRef,

    mutex: Vec<u8>,
	timer_id: Vec<u8>,
	delay: u16,
	res_num: u16,
	sfx_mod: SfxModule,
	mark_var: Vec<i16>,
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
            res_num: 0,
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

    pub fn load_sfx_module(&mut self, res_num: u16, delay: u16, pos: u8) -> Result<()> {
        // debug(DBG_SND, "SfxPlayer::loadSfxModule(0x%X, %d, %d)", resNum, delay, pos);
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
    
        // to avoid borrow checker complain
        let (state, res_type, me_offset) = {
            let me = &self.res.get().mem_entries[res_num as usize];
            (me.state, me.res_type, me.buf_offset)
        };

        if state == MemEntryState::Loaded && res_type == ResType::Music {
            self.res_num = res_num;
            self.sfx_mod = Default::default();
            self.sfx_mod.cur_order = pos;
            self.sfx_mod.num_order = self.res.get().from_mem_be_u16(me_offset as usize + 0x3E) as u8;
            // debug(DBG_SND, "SfxPlayer::loadSfxModule() curOrder = 0x%X numOrder = 0x%X", _sfxMod.curOrder, _sfxMod.numOrder);
            self.sfx_mod.order_table[..]
                .clone_from_slice(self.res.get().mem_to_slice(me_offset as usize + 0x40, 0x80));
            if delay == 0 {
                self.delay = self.res.get().from_mem_be_u16(me_offset as usize);
            } else {
                self.delay = delay;
            }
            self.delay *= 60 / 7050;
            self.sfx_mod.buf_offset = me_offset + 0xC0;
        //     debug(DBG_SND, "SfxPlayer::loadSfxModule() eventDelay = %d ms", _delay);
            self.prepare_instruments(me_offset as usize + 2)?;
        // } else {
        //     warning("SfxPlayer::loadSfxModule() ec=0x%X", 0xF8);
        }
        
        Ok(())
    }

    fn prepare_instruments(&mut self, mut offset: usize) -> Result<()> {
        // self.sfx_mod.samples.clear();

        for ins in &mut self.sfx_mod.samples {
            let res_num = self.res.get().from_mem_be_u16(offset as usize) as usize;
            offset += 2;

            if res_num != 0 {
                ins.volume = self.res.get().from_mem_be_u16(offset as usize);
                let me = &self.res.get().mem_entries[res_num];

                if me.state == MemEntryState::Loaded && me.res_type == ResType::Sound {
                    ins.buf_offset = me.buf_offset;
                    self.res.get_mut().memset(ins.buf_offset as usize + 8, 0, 4);
            //         debug(DBG_SND, "Loaded instrument 0x%X n=%d volume=%d", resNum, i, ins->volume);
                } else {
                    bail!("Error loading instrument {}", res_num);
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
        // self.timer_id = self.sys.get_mut().add_timer(self.delay as u32, &|_interval| { self.handle_events(); self.delay as u32 });
        todo!(); // TODO: add_timer
    }

    pub fn stop(&mut self) {
        // debug(DBG_SND, "SfxPlayer::stop()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        if self.res_num != 0 {
            self.res_num = 0;
            self.sys.get_mut().remove_timer(&self.timer_id);
        }
    }

    fn handle_events(&mut self) {
        todo!(); // TODO: implement
    }

    fn handle_pattern(&mut self, _channel: u8, _pattern_data: &[u8]) {
        todo!(); // TODO: implement
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) {
        self.sys.get_mut().lock_mutex(&self.mutex);

        ser.save_or_load_entries(self, Ver(2));

        self.sys.get_mut().unlock_mutex(&self.mutex);

        if ser.mode() == Mode::Load && self.res_num != 0 {
            let delay = self.delay;
            self.load_sfx_module(self.res_num, 0, self.sfx_mod.cur_order);
            self.delay = delay;
            // self.timer_id = self.sys.get_mut().add_timer(self.delay as u32, &|_interval| { self.handle_events(); self.delay as u32 });
            todo!(); // TODO: add_timer
        }
    }
}

// TODO: use proc_macro

impl AccessorWrap for SfxPlayer {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.delay.read(stream)?;
        self.res_num.read(stream)?;
        self.sfx_mod.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.delay.write(stream)?;
        self.res_num.write(stream)?;
        self.sfx_mod.write(stream)
    }

    fn size(&self) -> usize {
        self.delay.size() + self.res_num.size() + self.sfx_mod.size()
    }
}
