use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    file::File, memlist::MemEntryState, mixer::*, parts::*, reference::Ref, resource::ResourceRef,
    serializer::*, sfxplayer::SfxPlayer, staticres::*, system::*, video::Video,
};
use anyhow::Result;
use trace::trace;

trace::init_depth_var!();

pub const VM_NUM_THREADS: usize = 64;
const VM_NUM_VARIABLES: usize = 256;

#[derive(Clone, Copy)]
pub(crate) struct ThreadData {
    // This array is used:
    //     To save the channel's instruction pointer
    //     when the channel release control (this happens on a break).
    pub pc_offset: u16,
    //     When a setVec is requested for the next vm frame.
    pub requested_pc_offset: u16,

    pub cur_state_active: bool,
    pub requested_state_active: bool,
}

impl Default for ThreadData {
    fn default() -> Self {
        Self {
            pc_offset: 0xFFFF,
            requested_pc_offset: 0xFFFF,
            cur_state_active: true,
            requested_state_active: true,
        }
    }
}

impl fmt::Debug for ThreadData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!(
            "[{:04X}, {:04X}, {}, {}]",
            self.pc_offset,
            self.requested_pc_offset,
            self.cur_state_active,
            self.requested_state_active
        ))
    }
}

pub(crate) struct VmContext {
    sys: SystemRef,
    res: ResourceRef,
    mixer: MixerRef,
    player: SfxPlayer,

    script_stack_calls: [u16; VM_NUM_THREADS],
    fast_mode: bool,
    last_time_stamp: u32,

    pub video: Video,

    pub variables: [i16; VM_NUM_VARIABLES],
    pub threads_data: [ThreadData; VM_NUM_THREADS],
}

impl VmContext {
    pub fn new(sys: SystemRef, res: ResourceRef) -> Self {
        let mixer = Ref::new(Box::new(Mixer::new(sys.clone())));
        let player = SfxPlayer::new(mixer.clone(), res.clone(), sys.clone());
        let video = Video::new(res.clone(), sys.clone());

        Self {
            sys,
            res,
            mixer,
            player,
            script_stack_calls: [0; VM_NUM_THREADS],
            fast_mode: false,
            last_time_stamp: 0,
            video,
            variables: [0; VM_NUM_VARIABLES],
            threads_data: [Default::default(); VM_NUM_THREADS],
        }
    }

    pub fn toggle_fast_mode(&mut self) {
        self.fast_mode = !self.fast_mode;
    }

    #[trace]
    pub fn init(&mut self) {
        self.video.init();
        self.player.init();
        self.mixer.get_mut().init();

        self.variables = [0; VM_NUM_VARIABLES];
        self.variables[0x54] = 0x81;
        self.variables[VM_VARIABLE_RANDOM_SEED] = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Cannot get current time")
            .as_secs() as i16;

        self.fast_mode = false;
        // self.player.mark_var = &self.vm_variables[VM_VARIABLE_MUS_MARK]; // TODO: uncomment
    }

    #[trace]
    pub fn init_for_part(&mut self, part_id: u16) -> Result<()> {
        self.player.stop();
        self.mixer.get_mut().stop_all();

        //WTF is that ?
        self.variables[0xE4] = 0x14;

        self.res.get_mut().setup_part(part_id)?;

        //Set all thread to inactive (pc at 0xFFFF or 0xFFFE )
        self.threads_data = [Default::default(); VM_NUM_THREADS];

        self.threads_data[0].pc_offset = 0;

        Ok(())
    }

    pub fn inp_update_player(&mut self) {
        let mut sys = self.sys.get_mut();

        sys.process_events();

        if self.res.get().current_part_id() == GAME_PART10 {
            let c = sys.input().last_char;
            if c == 8 || /*c == 0xD |*/ c == 0 || (c >= b'a' && c <= b'z') {
                self.variables[VM_VARIABLE_LAST_KEYCHAR] = (c & !0x20) as i16;
                sys.input_mut().last_char = 0;
            }
        }

        let mut lr = 0;
        let mut m = 0;
        let mut ud = 0;

        if sys.input().dir_mask & DIR_RIGHT != 0 {
            lr = 1;
            m |= 1;
        }
        if sys.input().dir_mask & DIR_LEFT != 0 {
            lr = -1;
            m |= 2;
        }
        if sys.input().dir_mask & DIR_DOWN != 0 {
            ud = 1;
            m |= 4;
        }

        self.variables[VM_VARIABLE_HERO_POS_UP_DOWN] = ud;

        if sys.input().dir_mask & DIR_UP != 0 {
            self.variables[VM_VARIABLE_HERO_POS_UP_DOWN] = -1;
        }

        // inpJump
        if sys.input().dir_mask & DIR_UP != 0 {
            ud = -1;
            m |= 8;
        }

        self.variables[VM_VARIABLE_HERO_POS_JUMP_DOWN] = ud;
        self.variables[VM_VARIABLE_HERO_POS_LEFT_RIGHT] = lr;
        self.variables[VM_VARIABLE_HERO_POS_MASK] = m;

        let mut button = 0;

        // inpButton
        if sys.input().button {
            button = 1;
            m |= 0x80;
        }

        self.variables[VM_VARIABLE_HERO_ACTION] = button;
        self.variables[VM_VARIABLE_HERO_ACTION_POS_MASK] = m;
    }

    pub fn inp_handle_special_keys(&mut self) {
        let mut sys = self.sys.get_mut();
        let mut res = self.res.get_mut();

        if sys.input().pause {
            if res.current_part_id() != GAME_PART1 && res.current_part_id() != GAME_PART2 {
                sys.input_mut().pause = false;

                while !sys.input().pause {
                    sys.process_events();
                    sys.sleep(200);
                }
            }
            sys.input_mut().pause = false;
        }

        if sys.input().code {
            sys.input_mut().code = false;

            if res.current_part_id() != GAME_PART_LAST && res.current_part_id() != GAME_PART_FIRST {
                res.requested_next_part = Some(GAME_PART_LAST);
            }
        }

        // XXX
        // if self.vm_variables[0xC9] == 1 {
        //     warning("VirtualMachine::inp_handle_special_keys() unhandled case (self.vm_variables[0xC9] == 1)");
        // }
    }

    pub fn blit_framebuffer(&mut self, page_id: usize) {
        // debug(DBG_VM, "VirtualMachine::op_blit_framebuffer(%d)", page_id);
        self.inp_handle_special_keys();

        //Nasty hack....was this present in the original assembly  ??!!
        if self.res.get().current_part_id() == GAME_PART_FIRST && self.variables[0x67] == 1 {
            self.variables[0xDC] = 0x21;
        }

        if !self.fast_mode {
            let sys = self.sys.get();
            let delay = sys.get_timestamp() - self.last_time_stamp;
            let time_to_sleep = self.variables[VM_VARIABLE_PAUSE_SLICES] * 20 - delay as i16;

            // The bytecode will set self.vm_variables[VM_VARIABLE_PAUSE_SLICES] from 1 to 5
            // The virtual machine hence indicate how long the image should be displayed.

            //printf("self.vm_variables[VM_VARIABLE_PAUSE_SLICES]=%d\n",self.vm_variables[VM_VARIABLE_PAUSE_SLICES]);

            if time_to_sleep > 0 {
                //	printf("Sleeping for=%d\n",time_to_sleep);
                sys.sleep(time_to_sleep as u32);
            }

            self.last_time_stamp = sys.get_timestamp();
        }

        //WTF ?
        self.variables[0xF7] = 0;

        self.video.update_display(page_id);
    }

    pub fn play_sound(&mut self, res_id: u16, freq: u8, vol: u8, channel: u8) {
        // debug(DBG_SND, "snd_play_sound(0x%X, %d, %d, %d)", res_num, freq, vol, channel);

        let me = &self.res.get_mut().storage.mem_list.entries[res_id as usize];

        if me.state != MemEntryState::Loaded {
            return;
        }

        if vol == 0 {
            self.mixer.get_mut().stop_channel(channel);
        } else {
            let mut mc = MixerChunk {
                data: (me.buf_offset + 8) as u32, // skip header
                len: 0,                           //self.fetch_data_u16() * 2,
                loop_len: 0,                      //self.fetch_data_u16() * 2,
                ..Default::default()
            };
            if mc.loop_len != 0 {
                mc.loop_pos = mc.len;
            }
            assert!(freq < 40);
            self.mixer.get_mut().play_channel(
                channel & 3,
                mc,
                FREQUENCE_TABLE[freq as usize],
                u8::min(vol, 0x3F),
            );
        }
    }

    pub fn play_music(&mut self, res_id: u16, delay: u16, pos: u8) -> Result<()> {
        // debug(DBG_SND, "snd_play_music(0x%X, %d, %d)", res_num, delay, pos);

        if res_id != 0 {
            self.player.load_sfx_module(res_id, delay, pos)?;
            self.player.start();
        } else if delay != 0 {
            self.player.set_events_delay(delay);
        } else {
            self.player.stop();
        }

        Ok(())
    }

    pub fn update_mem_list(&mut self, res_id: u16) -> Result<()> {
        if res_id == 0 {
            self.player.stop();
            self.mixer.get_mut().stop_all();
            self.res.get_mut().invalidate_res();
        } else {
            self.res.get_mut().load_parts_or_mem_entry(res_id)?;
        }

        Ok(())
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        ser.save_or_load_entries(self, Ver(1))?;

        self.video.save_or_load(ser)?;

        if ser.mode() == Mode::Load {
            // mute
            self.player.stop();
            self.mixer.get_mut().stop_all();
        }

        self.player.save_or_load(ser)?;
        self.mixer.get_mut().save_or_load(ser)
    }
}

impl fmt::Debug for VmContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VmContext")
            .field(
                "script_stack_calls",
                &self
                    .script_stack_calls
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .field("fast_mode", &self.fast_mode)
            .field("last_time_stamp", &self.last_time_stamp)
            .field(
                "variables",
                &self
                    .variables
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .field(
                "threads_data",
                &self
                    .threads_data
                    .iter()
                    .map(|v| format!("{:?}", v))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            // .field("threads_data", &self.threads_data)
            .finish()
    }
}

// TODO: use proc_macro

impl AccessorWrap for VmContext {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.variables.read(stream)?;
        self.script_stack_calls.read(stream)
        // self.threads_data.read(stream)?;
        // self.vm_is_channel_active.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.variables.write(stream)?;
        self.script_stack_calls.write(stream)
        // self.threads_data.write(stream)?;
        // self.vm_is_channel_active.write(stream)
    }

    fn size(&self) -> usize {
        self.variables.size() + self.script_stack_calls.size()
        // + self.threads_data.size()
        // + self.vm_is_channel_active.size()
    }
}
