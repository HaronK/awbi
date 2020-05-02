use crate::{
    memlist::MemEntryState,
    mixer::{MixerChunk, MixerRef},
    parts::{GAME_PART1, GAME_PART2, GAME_PART_FIRST, GAME_PART_LAST},
    resource::ResourceRef,
    sfxplayer::SfxPlayerRef,
    staticres::*,
    system::SystemRef,
    video::Video,
    vm::{VM_NUM_THREADS, VM_NUM_VARIABLES, VM_VARIABLE_PAUSE_SLICES},
};
use anyhow::Result;

#[derive(Default, Clone, Copy)]
pub(crate) struct ThreadData {
    pub pc_offset: u16,
    pub requested_pc_offset: u16,
    pub cur_state_active: bool,
    pub requested_state_active: bool,
}

pub(crate) struct VmContext {
    sys: SystemRef,
    res: ResourceRef,
    mixer: MixerRef,
    player: SfxPlayerRef,

    fast_mode: bool,
    last_time_stamp: u32,

    pub video: Video,

    pub variables: [i16; VM_NUM_VARIABLES],
    pub threads_data: [ThreadData; VM_NUM_THREADS],
}

impl VmContext {
    pub fn new(
        sys: SystemRef,
        res: ResourceRef,
        mixer: MixerRef,
        player: SfxPlayerRef,
        video: Video,
    ) -> Self {
        Self {
            sys,
            res,
            mixer,
            player,
            fast_mode: false,
            last_time_stamp: 0,
            video,
            variables: [0; VM_NUM_VARIABLES],
            threads_data: [Default::default(); VM_NUM_THREADS],
        }
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
                res.requested_next_part = GAME_PART_LAST;
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

        let sys = self.sys.get();

        //Nasty hack....was this present in the original assembly  ??!!
        if self.res.get().current_part_id() == GAME_PART_FIRST && self.variables[0x67] == 1 {
            self.variables[0xDC] = 0x21;
        }

        if !self.fast_mode {
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

        let mut player = self.player.get_mut();
        if res_id != 0 {
            player.load_sfx_module(res_id, delay, pos)?;
            player.start();
        } else if delay != 0 {
            player.set_events_delay(delay);
        } else {
            player.stop();
        }

        Ok(())
    }

    pub fn update_mem_list(&mut self, res_id: u16) -> Result<()> {
        if res_id == 0 {
            self.player.get_mut().stop();
            self.mixer.get_mut().stop_all();
            self.res.get_mut().invalidate_res();
        } else {
            self.res.get_mut().load_parts_or_mem_entry(res_id)?;
        }

        Ok(())
    }
}
