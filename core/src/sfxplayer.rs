use crate::memlist::*;
use crate::mixer::*;
use crate::reference::*;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use crate::{file::*, slice_reader::SliceReader};
use anyhow::{bail, Result};

#[derive(Clone, Default)]
struct SfxInstrument {
    data: SliceReader,
    volume: u16,
}

struct SfxModule {
    data: SliceReader,
    cur_pos: u16,
    cur_order: u8,
    num_order: u8,
    order_table: [u8; 0x80],
    samples: Vec<SfxInstrument>,
}

impl Default for SfxModule {
    fn default() -> Self {
        Self {
            data: SliceReader::default(),
            cur_pos: 0,
            cur_order: 0,
            num_order: 0,
            order_table: [0; 0x80],
            samples: vec![Default::default(); 15],
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

#[derive(Default)]
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
    timer_id: TimerId,
    delay: u16,
    res_id: u16,
    sfx_mod: SfxModule,
    pub mark_var: Vec<i16>,
}

impl TimerHandler for SfxPlayer {
    fn handle(&mut self) -> u32 {
        self.handle_events();
        self.delay as u32
    }
}

impl SfxPlayer {
    pub fn new(mixer: MixerRef, res: ResourceRef, sys: SystemRef) -> Self {
        Self {
            mixer,
            res,
            sys,
            mutex: Vec::new(),
            timer_id: TimerId::default(),
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
        let src_me = &self.res.get().storage.mem_list.entries[res_id as usize];
        let mut me_offset = src_me.buf_offset as usize;

        if src_me.state == MemEntryState::Loaded && src_me.res_type == ResType::Music {
            self.res_id = res_id;
            self.sfx_mod = Default::default();
            self.sfx_mod.cur_order = pos;
            self.sfx_mod.num_order = src_me.from_buf_be_u16(me_offset + 0x3E) as u8;
            // debug(DBG_SND, "SfxPlayer::loadSfxModule() curOrder = 0x%X numOrder = 0x%X", self.sfx_mod.curOrder, self.sfx_mod.numOrder);

            // for i in 0..0x80 {
            //     self.sfx_mod.order_table[i] = me.from_buf_u8(me_offset + 0x40 + i);
            // }
            self.sfx_mod.order_table[..].clone_from_slice(src_me.to_slice(me_offset + 0x40, 0x80));

            if delay == 0 {
                self.delay = src_me.from_buf_be_u16(me_offset);
            } else {
                self.delay = delay;
            }
            self.delay *= 60 / 7050;
            self.sfx_mod.data = src_me.to_slice_end(me_offset + 0xC0).into();
            // debug(DBG_SND, "SfxPlayer::loadSfxModule() eventDelay = %d ms", _delay);

            // self.prepare_instruments(&src_me, me_offset + 2)?;
            // prepare instruments
            for ins in &mut self.sfx_mod.samples {
                let res_id = src_me.from_buf_be_u16(me_offset as usize) as usize;
                me_offset += 2;

                if res_id != 0 {
                    ins.volume = src_me.from_buf_be_u16(me_offset as usize);
                    let me = &self.res.get().storage.mem_list.entries[res_id];

                    if me.state == MemEntryState::Loaded && me.res_type == ResType::Sound {
                        let mut buf = me.buffer.clone();

                        // TODO: do it in idiomatic way
                        for i in 8..12 {
                            buf[i] = 0;
                        }

                        ins.data = buf.into();
                    //         debug(DBG_SND, "Loaded instrument 0x%X n=%d volume=%d", resNum, i, ins->volume);
                    } else {
                        bail!("Error loading instrument {}", res_id);
                    }
                }

                me_offset += 2; // skip volume
            }
        } else {
            //     warning("SfxPlayer::loadSfxModule() ec=0x%X", 0xF8);
        }

        Ok(())
    }

    // fn prepare_instruments(&mut self, src_me: &MemEntry, mut me_offset: usize) -> Result<()> {
    //     for ins in &mut self.sfx_mod.samples {
    //         let res_id = src_me.from_buf_be_u16(me_offset as usize) as usize;
    //         me_offset += 2;

    //         if res_id != 0 {
    //             ins.volume = src_me.from_buf_be_u16(me_offset as usize);
    //             let me = &self.res.get().storage.mem_list.entries[res_id];

    //             if me.state == MemEntryState::Loaded && me.res_type == ResType::Sound {
    //                 ins.buf_offset = me.buf_offset as u16;
    //                 self.res.get_mut().memset(ins.buf_offset as usize + 8, 0, 4);
    //             //         debug(DBG_SND, "Loaded instrument 0x%X n=%d volume=%d", resNum, i, ins->volume);
    //             } else {
    //                 bail!("Error loading instrument {}", res_id);
    //             }
    //         }

    //         me_offset += 2; // skip volume
    //     }

    //     Ok(())
    // }

    pub fn start(&mut self) {
        // debug(DBG_SND, "SfxPlayer::start()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.sfx_mod.cur_pos = 0;
        self.timer_id = self
            .sys
            .get_mut()
            .add_timer(self.delay as u32, &|_interval, handler| {
                handler.handle()
                // self.delay as u32
            }); // TODO: uncomment
    }

    pub fn stop(&mut self) {
        // debug(DBG_SND, "SfxPlayer::stop()");
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        if self.res_id != 0 {
            self.res_id = 0;
            self.sys.get_mut().remove_timer(self.timer_id);
        }
    }

    fn handle_events(&mut self) {
        // todo!(); // TODO: implement
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        let mut order = self.sfx_mod.order_table[self.sfx_mod.cur_order as usize] as u16;
        let mut pattern_data_idx: u16 = self.sfx_mod.cur_pos + order * 1024;

        for ch in 0..4 {
            self.handle_pattern(ch, pattern_data_idx);
            pattern_data_idx += 4;
        }

        self.sfx_mod.cur_pos += 4 * 4;
        // debug(DBG_SND, "SfxPlayer::handleEvents() order = 0x%X curPos = 0x%X", order, self.sfx_mod.curPos);
        if self.sfx_mod.cur_pos >= 1024 {
            self.sfx_mod.cur_pos = 0;
            order = self.sfx_mod.cur_pos + 1;

            if order == self.sfx_mod.num_order as _ {
                self.res_id = 0;
                self.sys.get_mut().remove_timer(self.timer_id);
                self.mixer.get_mut().stop_all();
            }

            self.sfx_mod.cur_pos = order;
        }
    }

    fn handle_pattern(&mut self, channel: u8, _pattern_data_idx: u16) {
        let mut pat = SfxPattern::default();

        pat.note_1 = self.sfx_mod.data.read_u16();
        pat.note_2 = self.sfx_mod.data.read_u16();

        if pat.note_1 != 0xFFFD {
            let sample = ((pat.note_2 & 0xF000) >> 12) as usize;
            if sample != 0 {
                let instrument = &mut self.sfx_mod.samples[sample - 1];
                let slice_reader = &mut instrument.data;

                if !slice_reader.is_empty() {
                    // debug(DBG_SND, "SfxPlayer::handlePattern() preparing sample %d", sample);
                    pat.sample_volume = instrument.volume;
                    pat.sample_start = 8;
                    pat.sample_buffer = slice_reader.get_data().into();
                    pat.sample_len = slice_reader.read_u16() * 2;
                    let loop_len = slice_reader.read_u16() * 2;

                    if loop_len != 0 {
                        pat.loop_pos = pat.sample_len;
                        pat.loop_data = slice_reader.get_data().into();
                        pat.loop_len = loop_len;
                    } else {
                        pat.loop_pos = 0;
                        pat.loop_len = 0;
                    }

                    let mut m = pat.sample_volume as i16;
                    let effect = (pat.note_2 & 0x0F00) >> 8;

                    if effect == 5 {
                        // volume up
                        let volume = (pat.note_2 & 0xFF) as i16;
                        m += volume;
                        if m > 0x3F {
                            m = 0x3F;
                        }
                    } else if effect == 6 {
                        // volume down
                        let volume = (pat.note_2 & 0xFF) as i16;
                        m -= volume;
                        if m < 0 {
                            m = 0;
                        }
                    }

                    self.mixer.get_mut().set_channel_volume(channel, m as u8);
                    pat.sample_volume = m as u16;
                }
            }
        }

        if pat.note_1 == 0xFFFD {
            // debug(DBG_SND, "SfxPlayer::handlePattern() _scriptVars[0xF4] = 0x%X", pat.note_2);
            self.mark_var[0] = pat.note_2 as i16;
        } else if pat.note_1 != 0 {
            if pat.note_1 == 0xFFFE {
                self.mixer.get_mut().stop_channel(channel);
            } else if !pat.sample_buffer.is_empty() {
                let mut mc = MixerChunk::default();

                mc.data = pat.sample_buffer[pat.sample_start as usize..].into();
                mc.len = pat.sample_len;
                mc.loop_pos = pat.loop_pos;
                mc.loop_len = pat.loop_len;
                assert!(pat.note_1 >= 0x37 && pat.note_1 < 0x1000);
                // convert amiga period value to hz
                let freq = 7159092 / (pat.note_1 as u32 * 2);
                // debug(DBG_SND, "SfxPlayer::handlePattern() adding sample freq = 0x%X", freq);
                self.mixer.get_mut().play_channel(
                    channel,
                    mc,
                    freq as u16,
                    pat.sample_volume as u8,
                );
            }
        }
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
