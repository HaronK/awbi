use crate::file::*;
use crate::mixer::*;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use anyhow::Result;

#[derive(Default)]
struct SfxInstrument {
	data: Vec<u8>,
	volume: u16,
}

#[derive(Default)]
struct SfxModule {
	data: Vec<u8>,
	cur_pos: u16,
	cur_order: u8,
	num_order: u8,
	order_table: Vec<u8>, //[0x80];
	samples: Vec<SfxInstrument>, //[15];
}

impl SfxModule {
    fn new() -> Self {
        Self {
            order_table: vec![0; 0x80],
            ..Default::default()
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

    fn init(&mut self) {
        self.mutex = self.sys.get_mut().create_mutex();
    }

    fn free(&mut self) {
        self.stop();
        self.sys.get_mut().destroy_mutex(&self.mutex);

    }

    fn set_events_delay(&mut self, delay: u16) {
        // debug(DBG_SND, "SfxPlayer::setEventsDelay(%d)", delay);
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.delay = delay * 60 / 7050;
    }

    fn load_sfx_module(&mut self, res_num: u16, delay: u16, pos: u8) {
        todo!(); // TODO: implement
    }

    fn prepare_instruments(&mut self, p: &[u8]) {
        todo!(); // TODO: implement
    }

    fn start(&mut self) {
        // debug(DBG_SND, "SfxPlayer::start()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.sfx_mod.cur_pos = 0;
        // self.timer_id = self.sys.get_mut().add_timer(self.delay as u32, &|_interval| { self.handle_events(); self.delay as u32 });
        todo!(); // TODO: add_timer
    }

    fn stop(&mut self) {
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

    fn handle_pattern(&mut self, channel: u8, pattern_data: &[u8]) {
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
