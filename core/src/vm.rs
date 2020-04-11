use crate::file::*;
use crate::mixer::*;
use crate::parts::*;
use crate::resource::*;
use crate::serializer::*;
use crate::sfxplayer::*;
use crate::staticres::*;
use crate::system::*;
use crate::video::*;
use anyhow::Result;

const VM_NUM_THREADS: usize = 64;
const VM_NUM_VARIABLES: usize = 256;
const VM_NO_SETVEC_REQUESTED: u16 = 0xFFFF;
const VM_INACTIVE_THREAD: u16 = 0xFFFF;

const VM_VARIABLE_RANDOM_SEED: usize = 0x3C;
const VM_VARIABLE_LAST_KEYCHAR: usize = 0xDA;
const VM_VARIABLE_HERO_POS_UP_DOWN: usize = 0xE5;
const VM_VARIABLE_MUS_MARK: usize = 0xF4;
const VM_VARIABLE_SCROLL_Y: usize = 0xF9; // = 239
const VM_VARIABLE_HERO_ACTION: usize = 0xFA;
const VM_VARIABLE_HERO_POS_JUMP_DOWN: usize = 0xFB;
const VM_VARIABLE_HERO_POS_LEFT_RIGHT: usize = 0xFC;
const VM_VARIABLE_HERO_POS_MASK: usize = 0xFD;
const VM_VARIABLE_HERO_ACTION_POS_MASK: usize = 0xFE;
const VM_VARIABLE_PAUSE_SLICES: usize = 0xFF;

//For self.threads_data navigation
const PC_OFFSET: usize = 0;
const REQUESTED_PC_OFFSET: usize = 1;
const NUM_DATA_FIELDS: usize = 2;

//For self.vm_is_channel_active navigation
const CUR_STATE: usize = 0;
const REQUESTED_STATE: usize = 1;
const NUM_THREAD_FIELDS: usize = 2;

const COLOR_BLACK: u8 = 0xFF;
const DEFAULT_ZOOM: u16 = 0x0040;

pub(crate) struct VirtualMachine {
    // The type of entries in opcodeTable. This allows "fast" branching
    // typedef void (VirtualMachine::*OpcodeStub)();
    // static const OpcodeStub opcodeTable[];

    //This table is used to play a sound
    // static const let frequenceTable[];
    mixer: MixerRef,
    res: ResourceRef,
    player: SfxPlayer,
    video: Video,
    sys: SystemRef,

    vm_variables: [i16; VM_NUM_VARIABLES],
    script_stack_calls: [u16; VM_NUM_THREADS],

    threads_data: [[u16; VM_NUM_THREADS]; NUM_DATA_FIELDS],

    // This array is used:
    //     0 to save the channel's instruction pointer
    //     when the channel release control (this happens on a break).

    //     1 When a setVec is requested for the next vm frame.
    vm_is_channel_active: [[u8; VM_NUM_THREADS]; NUM_THREAD_FIELDS],

    data_page_idx: usize,
    data_page_offset: usize,
    stack_ptr: usize,
    goto_next_thread: bool,
    pub fast_mode: bool,

    last_time_stamp: u32,
}

impl VirtualMachine {
    pub fn new(mixer: MixerRef, res: ResourceRef, sys: SystemRef) -> Self {
        let code_idx = res.get().seg_code_idx();
        let player = SfxPlayer::new(mixer.clone(), res.clone(), sys.clone());
        let video = Video::new(res.clone(), sys.clone());

        Self {
            mixer,
            res,
            player,
            video,
            sys,

            vm_variables: [0; VM_NUM_VARIABLES],
            script_stack_calls: [0; VM_NUM_THREADS],
            threads_data: [[0; VM_NUM_THREADS]; NUM_DATA_FIELDS],
            vm_is_channel_active: [[0; VM_NUM_THREADS]; NUM_THREAD_FIELDS],
            data_page_idx: code_idx,
            data_page_offset: 0,
            stack_ptr: 0,
            goto_next_thread: false,
            fast_mode: false,

            last_time_stamp: 0,
        }
    }

    pub fn init(&mut self) {
        self.video.init();
        self.player.init();

        self.vm_variables = [0; VM_NUM_VARIABLES];
        self.vm_variables[0x54] = 0x81;
        // self.vm_variables[VM_VARIABLE_RANDOM_SEED] = time(0); // TODO: uncomment

        self.fast_mode = false;
        // self.player.get_mut().mark_var = &self.vm_variables[VM_VARIABLE_MUS_MARK]; // TODO: uncomment

        self.last_time_stamp = 0;
        todo!();
    }

    fn page_offset(&self) -> usize {
        self.data_page_idx + self.data_page_offset
    }

    fn fetch_data_u8(&mut self) -> u8 {
        let res = self.res.get().from_mem_u8(self.page_offset());
        self.data_page_offset += 1;
        res
    }

    fn fetch_data_u16(&mut self) -> u16 {
        let res = self.res.get().from_mem_be_u16(self.page_offset());
        self.data_page_offset += 2;
        res
    }

    fn op_mov_const(&mut self) {
        let variable_id = self.fetch_data_u8() as usize;
        let value = self.fetch_data_u16() as i16;
        // debug(DBG_VM, "VirtualMachine::op_movConst(0x%02X, %d)", variable_id, value);
        self.vm_variables[variable_id] = value;
    }

    fn op_mov(&mut self) {
        let dst_variable_id = self.fetch_data_u8() as usize;
        let src_variable_id = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_mov(0x%02X, 0x%02X)", dst_variable_id, src_variable_id);
        self.vm_variables[dst_variable_id] = self.vm_variables[src_variable_id];
    }

    fn op_add(&mut self) {
        let dst_variable_id = self.fetch_data_u8() as usize;
        let src_variable_id = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_add(0x%02X, 0x%02X)", dst_variable_id, src_variable_id);
        self.vm_variables[dst_variable_id] += self.vm_variables[src_variable_id];
    }

    fn op_add_const(&mut self) {
        if self.res.get().current_part_id() == 0x3E86 && self.data_page_offset == 0x6D48 {
            // warning("VirtualMachine::op_add_const() hack for non-stop looping gun sound bug");
            // the script 0x27 slot 0x17 doesn't stop the gun sound from looping, I
            // don't really know why ; for now, let's play the 'stopping sound' like
            // the other scripts do
            //  (0x6D43) jmp(0x6CE5)
            //  (0x6D46) break
            //  (0x6D47) VAR(6) += -50
            self.snd_play_sound(0x5B, 1, 64, 1);
        }

        let variable_id = self.fetch_data_u8() as usize;
        let value = self.fetch_data_u16() as i16;
        // debug(DBG_VM, "VirtualMachine::op_add_const(0x%02X, %d)", variable_id, value);
        self.vm_variables[variable_id] += value;
    }

    fn op_call(&mut self) {
        let offset = self.fetch_data_u16() as usize;
        let sp = self.stack_ptr;

        // debug(DBG_VM, "VirtualMachine::op_call(0x%X)", offset);
        self.script_stack_calls[sp] = self.data_page_offset as u16;
        // if (self.stack_ptr == 0xFF) {
        //     error("VirtualMachine::op_call() ec=0x%X stack overflow", 0x8F);
        // }
        self.stack_ptr += 1;
        self.data_page_offset = offset;
    }

    fn op_ret(&mut self) {
        // debug(DBG_VM, "VirtualMachine::op_ret()");
        // if (self.stack_ptr == 0) {
        //     error("VirtualMachine::op_ret() ec=0x%X stack underflow", 0x8F);
        // }
        self.stack_ptr -= 1;
        let sp = self.stack_ptr;
        self.data_page_offset = self.script_stack_calls[sp] as usize;
    }

    fn op_pause_thread(&mut self) {
        // debug(DBG_VM, "VirtualMachine::op_pause_thread()");
        self.goto_next_thread = true;
    }

    fn op_jmp(&mut self) {
        let pc_offset = self.fetch_data_u16() as usize;
        // debug(DBG_VM, "VirtualMachine::op_jmp(0x%02X)", pc_offset);
        self.data_page_offset = pc_offset;
    }

    fn op_set_set_vect(&mut self) {
        let thread_id = self.fetch_data_u8() as usize;
        let pc_offset_requested = self.fetch_data_u16();
        // debug(DBG_VM, "VirtualMachine::op_set_set_vect(0x%X, 0x%X)", thread_id,pc_offset_requested);
        self.threads_data[REQUESTED_PC_OFFSET][thread_id] = pc_offset_requested;
    }

    fn op_jnz(&mut self) {
        let i = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_jnz(0x%02X)", i);
        self.vm_variables[i] -= 1;
        if self.vm_variables[i] != 0 {
            self.op_jmp();
        } else {
            let _ = self.fetch_data_u16();
        }
    }

    // #define BYPASS_PROTECTION
    fn op_cond_jmp(&mut self) {
        //printf("Jump : %X \n",self.page_offset()-self.res.get().seg_code_idx());
        // //FCS Whoever wrote this is patching the bytecode on the fly. This is ballzy !!
        // #ifdef BYPASS_PROTECTION

        //     if (self.res.get().current_part_id == GAME_PART_FIRST && self.page_offset() == self.res.get().seg_code_idx() + 0xCB9) {

        //         // (0x0CB8) condJmp(0x80, VAR(41), VAR(30), 0xCD3)
        //         *(_scriptPtr.pc + 0x00) = 0x81;
        //         *(_scriptPtr.pc + 0x03) = 0x0D;
        //         *(_scriptPtr.pc + 0x04) = 0x24;
        //         // (0x0D4E) condJmp(0x4, VAR(50), 6, 0xDBC)
        //         *(_scriptPtr.pc + 0x99) = 0x0D;
        //         *(_scriptPtr.pc + 0x9A) = 0x5A;
        //         printf("VirtualMachine::op_condJmp() bypassing protection");
        //         printf("bytecode has been patched/n");

        //         //this->bypassProtection() ;
        //     }

        // #endif

        let opcode = self.fetch_data_u8();
        let i = self.fetch_data_u8() as usize;
        let b = self.vm_variables[i];
        let c = self.fetch_data_u8();
        let a = if opcode & 0x80 != 0 {
            self.vm_variables[c as usize]
        } else if opcode & 0x40 != 0 {
            (c as i16) * 256 + self.fetch_data_u8() as i16
        } else {
            c as i16
        };
        // debug(DBG_VM, "VirtualMachine::op_condJmp(%d, 0x%02X, 0x%02X)", opcode, b, a);

        // Check if the conditional value is met.
        let expr = match opcode & 7 {
            0 => b == a, // jz
            1 => b != a, // jnz
            2 => b > a,  // jg
            3 => b >= a, // jge
            4 => b < a,  // jl
            5 => b <= a, // jle
            _ => false, // warning("VirtualMachine::op_condJmp() invalid condition %d", (opcode & 7));
        };

        if expr {
            self.op_jmp();
        } else {
            let _ = self.fetch_data_u16();
        }
    }

    fn op_set_palette(&mut self) {
        let palette_id = self.fetch_data_u16();
        // debug(DBG_VM, "VirtualMachine::op_changePalette(%d)", palette_id);
        self.video.palette_id_requested = (palette_id >> 8) as u8;
    }

    fn op_reset_thread(&mut self) {
        let thread_id = self.fetch_data_u8() as usize;
        let mut i = self.fetch_data_u8() as usize;

        // FCS: WTF, this is cryptic as hell !!
        //let n = (i & 0x3F) - thread_id;  //0x3F = 0011 1111
        // The following is so much clearer

        //Make sure i within [0-VM_NUM_THREADS-1]
        i &= VM_NUM_THREADS - 1;

        if i < thread_id {
            // warning("VirtualMachine::op_reset_thread() ec=0x%X (n < 0)", 0x880);
            return;
        }

        let n = i - thread_id + 1;
        let a = self.fetch_data_u8();

        // debug(DBG_VM, "VirtualMachine::op_reset_thread(%d, %d, %d)", thread_id, i, a);

        if a == 2 {
            for data in &mut self.threads_data[REQUESTED_PC_OFFSET][thread_id..thread_id + n] {
                *data = 0xFFFE;
            }
        } else if a < 2 {
            for data in &mut self.vm_is_channel_active[REQUESTED_STATE][thread_id..thread_id + n] {
                *data = a;
            }
        }
    }

    fn op_select_video_page(&mut self) {
        let frame_buffer_id = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_select_video_page(%d)", frame_buffer_id);
        self.video.change_page_off1(frame_buffer_id);
    }

    fn op_fill_video_page(&mut self) {
        let page_id = self.fetch_data_u8() as usize;
        let color = self.fetch_data_u8();
        // debug(DBG_VM, "VirtualMachine::op_fill_video_page(%d, %d)", page_id, color);
        self.video.fill_page(page_id, color);
    }

    fn op_copy_video_page(&mut self) {
        let src_page_id = self.fetch_data_u8() as usize;
        let dst_page_id = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_copy_video_page(%d, %d)", src_page_id, dst_page_id);
        self.video.copy_page(
            src_page_id,
            dst_page_id,
            self.vm_variables[VM_VARIABLE_SCROLL_Y],
        );
    }

    fn op_blit_framebuffer(&mut self) {
        let page_id = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_blit_framebuffer(%d)", page_id);
        self.inp_handle_special_keys();

        //Nasty hack....was this present in the original assembly  ??!!
        if self.res.get().current_part_id() == GAME_PART_FIRST && self.vm_variables[0x67] == 1 {
            self.vm_variables[0xDC] = 0x21;
        }

        if !self.fast_mode {
            let delay = self.sys.get().get_timestamp() - self.last_time_stamp;
            let time_to_sleep = self.vm_variables[VM_VARIABLE_PAUSE_SLICES] * 20 - delay as i16;

            // The bytecode will set self.vm_variables[VM_VARIABLE_PAUSE_SLICES] from 1 to 5
            // The virtual machine hence indicate how long the image should be displayed.

            //printf("self.vm_variables[VM_VARIABLE_PAUSE_SLICES]=%d\n",self.vm_variables[VM_VARIABLE_PAUSE_SLICES]);

            if time_to_sleep > 0 {
                //	printf("Sleeping for=%d\n",time_to_sleep);
                self.sys.get().sleep(time_to_sleep as u32);
            }

            self.last_time_stamp = self.sys.get().get_timestamp();
        }

        //WTF ?
        self.vm_variables[0xF7] = 0;

        self.video.update_display(page_id);
    }

    fn op_kill_thread(&mut self) {
        // debug(DBG_VM, "VirtualMachine::op_kill_thread()");
        self.data_page_offset = 0xFFFF;
        self.goto_next_thread = true;
    }

    fn op_draw_string(&mut self) {
        let string_id = self.fetch_data_u16();
        let x = self.fetch_data_u8() as u16;
        let y = self.fetch_data_u8() as u16;
        let color = self.fetch_data_u8();

        // debug(DBG_VM, "VirtualMachine::op_draw_string(0x%03X, %d, %d, %d)", string_id, x, y, color);

        self.video.draw_string(color, x, y, string_id);
    }

    fn op_sub(&mut self) {
        let i = self.fetch_data_u8() as usize;
        let j = self.fetch_data_u8() as usize;
        // debug(DBG_VM, "VirtualMachine::op_sub(0x%02X, 0x%02X)", i, j);
        self.vm_variables[i] -= self.vm_variables[j];
    }

    fn op_and(&mut self) {
        let variable_id = self.fetch_data_u8() as usize;
        let n = self.fetch_data_u16() as i16;
        // debug(DBG_VM, "VirtualMachine::op_and(0x%02X, %d)", variable_id, n);
        self.vm_variables[variable_id] &= n;
    }

    fn op_or(&mut self) {
        let variable_id = self.fetch_data_u8() as usize;
        let value = self.fetch_data_u16() as i16;
        // debug(DBG_VM, "VirtualMachine::op_or(0x%02X, %d)", variable_id, value);
        self.vm_variables[variable_id] |= value;
    }

    fn op_shl(&mut self) {
        let variable_id = self.fetch_data_u8() as usize;
        let left_shift_value = self.fetch_data_u16();
        // debug(DBG_VM, "VirtualMachine::op_shl(0x%02X, %d)", variable_id, left_shift_value);
        self.vm_variables[variable_id] <<= left_shift_value;
    }

    fn op_shr(&mut self) {
        let variable_id = self.fetch_data_u8() as usize;
        let right_shift_value = self.fetch_data_u16();
        // debug(DBG_VM, "VirtualMachine::op_shr(0x%02X, %d)", variable_id, right_shift_value);
        self.vm_variables[variable_id] >>= right_shift_value;
    }

    fn op_play_sound(&mut self) {
        let resource_id = self.fetch_data_u16() as usize;
        let freq = self.fetch_data_u8();
        let vol = self.fetch_data_u8();
        let channel = self.fetch_data_u8();
        // debug(DBG_VM, "VirtualMachine::op_play_sound(0x%X, %d, %d, %d)", resource_id, freq, vol, channel);
        self.snd_play_sound(resource_id, freq, vol, channel);
    }

    fn op_update_mem_list(&mut self) -> Result<()> {
        let resource_id = self.fetch_data_u16();
        // debug(DBG_VM, "VirtualMachine::op_update_mem_list(%d)", resource_id);

        if resource_id == 0 {
            self.player.stop();
            self.mixer.get_mut().stop_all();
            self.res.get_mut().invalidate_res();
        } else {
            self.res.get_mut().load_parts_or_mem_entry(resource_id)?;
        }

        Ok(())
    }

    fn op_play_music(&mut self) -> Result<()> {
        let res_num = self.fetch_data_u16();
        let delay = self.fetch_data_u16();
        let pos = self.fetch_data_u8();
        // debug(DBG_VM, "VirtualMachine::op_play_music(0x%X, %d, %d)", res_num, delay, pos);
        self.snd_play_music(res_num, delay, pos)
    }

    pub fn init_for_part(&mut self, part_id: u16) -> Result<()> {
        self.player.stop();
        self.mixer.get_mut().stop_all();

        //WTF is that ?
        self.vm_variables[0xE4] = 0x14;

        self.res.get_mut().setup_part(part_id)?;

        //Set all thread to inactive (pc at 0xFFFF or 0xFFFE )
        self.threads_data = [[0xFF; VM_NUM_THREADS]; NUM_DATA_FIELDS];

        self.vm_is_channel_active = [[0; VM_NUM_THREADS]; NUM_THREAD_FIELDS];

        self.threads_data[PC_OFFSET][0] = 0;

        Ok(())
    }

    /*
         This is called every frames in the infinite loop.
    */
    pub fn check_thread_requests(&mut self) -> Result<()> {
        //Check if a part switch has been requested.
        let requested_next_part = self.res.get().requested_next_part;
        if requested_next_part != 0 {
            self.init_for_part(requested_next_part)?;
            self.res.get_mut().requested_next_part = 0;
        }

        // Check if a state update has been requested for any thread during the previous VM execution:
        //      - Pause
        //      - Jump

        // JUMP:
        // Note: If a jump has been requested, the jump destination is stored
        // in self.threads_data[REQUESTED_PC_OFFSET]. Otherwise self.threads_data[REQUESTED_PC_OFFSET] == 0xFFFF

        // PAUSE:
        // Note: If a pause has been requested it is stored in  self.vm_is_channel_active[REQUESTED_STATE][i]

        for thread_id in 0..VM_NUM_THREADS {
            self.vm_is_channel_active[CUR_STATE][thread_id] =
                self.vm_is_channel_active[REQUESTED_STATE][thread_id];

            let n = self.threads_data[REQUESTED_PC_OFFSET][thread_id];

            if n != VM_NO_SETVEC_REQUESTED {
                self.threads_data[PC_OFFSET][thread_id] =
                    if n == 0xFFFE { VM_INACTIVE_THREAD } else { n };
                self.threads_data[REQUESTED_PC_OFFSET][thread_id] = VM_NO_SETVEC_REQUESTED;
            }
        }

        Ok(())
    }

    pub fn host_frame(&mut self) {
        // Run the Virtual Machine for every active threads (one vm frame).
        // Inactive threads are marked with a thread instruction pointer set to 0xFFFF (VM_INACTIVE_THREAD).
        // A thread must feature a break opcode so the interpreter can move to the next thread.

        for thread_id in 0..VM_NUM_THREADS {
            if self.vm_is_channel_active[CUR_STATE][thread_id] != 0 {
                continue;
            }

            let n = self.threads_data[PC_OFFSET][thread_id];

            if n != VM_INACTIVE_THREAD {
                // Set the script pointer to the right location.
                // script pc is used in execute_thread in order
                // to get the next opcode.
                self.data_page_offset = n as usize;
                self.stack_ptr = 0;

                self.goto_next_thread = false;
                // debug(DBG_VM, "VirtualMachine::host_frame() i=0x%02X n=0x%02X *p=0x%02X", thread_id, n, *self.page_offset());
                self.execute_thread();

                //Since .pc is going to be modified by this next loop iteration, we need to save it.
                self.threads_data[PC_OFFSET][thread_id] = self.data_page_offset as u16;

                // debug(DBG_VM, "VirtualMachine::host_frame() i=0x%02X pos=0x%X", thread_id, self.threads_data[PC_OFFSET][thread_id]);
                if self.sys.get().input().quit {
                    break;
                }
            }
        }
    }

    fn execute_thread(&mut self) {
        while !self.goto_next_thread {
            let opcode = self.fetch_data_u8();

            // 1000 0000 is set
            if opcode & 0x80 != 0 {
                let off = (((opcode << 8) | self.fetch_data_u8()) * 2) as usize;
                self.res.get_mut().set_use_seg_video2(false);
                let mut x = self.fetch_data_u8() as i16;
                let mut y = self.fetch_data_u8() as i16;
                let h = y - 199;
                if h > 0 {
                    y = 199;
                    x += h;
                }
                // debug(DBG_VIDEO, "vid_opcd_0x80 : opcode=0x%X off=0x%X x=%d y=%d", opcode, off, x, y);

                // This switch the polygon database to "cinematic" and probably draws a black polygon
                // over all the screen.
                self.video
                    .set_data_page(self.res.get().seg_cinematic_idx(), off);
                self.video
                    .read_and_draw_polygon(COLOR_BLACK, DEFAULT_ZOOM, &Point::new(x, y));

                continue;
            }

            // 0100 0000 is set
            if opcode & 0x40 != 0 {
                let off = (self.fetch_data_u16() * 2) as usize;
                let mut x = self.fetch_data_u8() as i16;

                self.res.get_mut().set_use_seg_video2(false);

                if opcode & 0x20 == 0 {
                    // 0001 0000 is set
                    if opcode & 0x10 == 0 {
                        x = (x << 8) | self.fetch_data_u8() as i16;
                    } else {
                        x = self.vm_variables[x as usize];
                    }
                } else if opcode & 0x10 != 0 {
                    // 0001 0000 is set
                    x += 0x100;
                }

                let mut y = self.fetch_data_u8() as i16;

                // 0000 1000 is set
                if opcode & 8 == 0 {
                    // 0000 0100 is set
                    if opcode & 4 == 0 {
                        y = (y << 8) | self.fetch_data_u8() as i16;
                    } else {
                        y = self.vm_variables[y as usize];
                    }
                }

                let mut zoom = self.fetch_data_u8() as u16;

                if opcode & 2 == 0 {
                    // 0000 0010 is set
                    if opcode & 1 == 0 {
                        // 0000 0001 is set
                        self.data_page_offset -= 1;
                        zoom = DEFAULT_ZOOM;
                    } else {
                        zoom = self.vm_variables[zoom as usize] as u16;
                    }
                } else if opcode & 1 != 0 {
                    // 0000 0001 is set
                    self.res.get_mut().set_use_seg_video2(true);
                    self.data_page_offset -= 1;
                    zoom = DEFAULT_ZOOM;
                }
                // debug(DBG_VIDEO, "vid_opcd_0x40 : off=0x%X x=%d y=%d", off, x, y);

                self.video.set_data_page(
                    if self.res.get().use_seg_video2() {
                        self.res.get().seg_video2_idx()
                    } else {
                        self.res.get().seg_cinematic_idx()
                    },
                    off,
                );
                self.video
                    .read_and_draw_polygon(0xFF, zoom, &Point::new(x, y));

                continue;
            }

            if opcode > 0x1A {
                // error("VirtualMachine::execute_thread() ec=0x%X invalid opcode=0x%X", 0xFFF, opcode);
            } else {
                // (this->*opcodeTable[opcode])();
                todo!(); // TODO:
            }
        }
    }

    pub fn inp_update_player(&mut self) {
        self.sys.get_mut().process_events();

        if self.res.get().current_part_id() == 0x3E89 {
            let c = self.sys.get().input().last_char;
            if c == 8 || /*c == 0xD |*/ c == 0 || (c >= b'a' && c <= b'z') {
                self.vm_variables[VM_VARIABLE_LAST_KEYCHAR] = (c & !0x20) as i16;
                self.sys.get_mut().input_mut().last_char = 0;
            }
        }

        let mut lr = 0;
        let mut m = 0;
        let mut ud = 0;

        if self.sys.get().input().dir_mask & DIR_RIGHT != 0 {
            lr = 1;
            m |= 1;
        }
        if self.sys.get().input().dir_mask & DIR_LEFT != 0 {
            lr = -1;
            m |= 2;
        }
        if self.sys.get().input().dir_mask & DIR_DOWN != 0 {
            ud = 1;
            m |= 4;
        }

        self.vm_variables[VM_VARIABLE_HERO_POS_UP_DOWN] = ud;

        if self.sys.get().input().dir_mask & DIR_UP != 0 {
            self.vm_variables[VM_VARIABLE_HERO_POS_UP_DOWN] = -1;
        }

        // inpJump
        if self.sys.get().input().dir_mask & DIR_UP != 0 {
            ud = -1;
            m |= 8;
        }

        self.vm_variables[VM_VARIABLE_HERO_POS_JUMP_DOWN] = ud;
        self.vm_variables[VM_VARIABLE_HERO_POS_LEFT_RIGHT] = lr;
        self.vm_variables[VM_VARIABLE_HERO_POS_MASK] = m;

        let mut button = 0;

        // inpButton
        if self.sys.get().input().button {
            button = 1;
            m |= 0x80;
        }

        self.vm_variables[VM_VARIABLE_HERO_ACTION] = button;
        self.vm_variables[VM_VARIABLE_HERO_ACTION_POS_MASK] = m;
    }

    fn inp_handle_special_keys(&mut self) {
        if self.sys.get().input().pause {
            if self.res.get().current_part_id() != GAME_PART1
                && self.res.get().current_part_id() != GAME_PART2
            {
                self.sys.get_mut().input_mut().pause = false;

                while !self.sys.get().input().pause {
                    self.sys.get_mut().process_events();
                    self.sys.get().sleep(200);
                }
            }
            self.sys.get_mut().input_mut().pause = false;
        }

        if self.sys.get().input().code {
            self.sys.get_mut().input_mut().code = false;

            if self.res.get().current_part_id() != GAME_PART_LAST
                && self.res.get().current_part_id() != GAME_PART_FIRST
            {
                self.res.get_mut().requested_next_part = GAME_PART_LAST;
            }
        }

        // XXX
        // if self.vm_variables[0xC9] == 1 {
        //     warning("VirtualMachine::inp_handle_special_keys() unhandled case (self.vm_variables[0xC9] == 1)");
        // }
    }

    fn snd_play_sound(&mut self, res_num: usize, freq: u8, vol: u8, channel: u8) {
        // debug(DBG_SND, "snd_play_sound(0x%X, %d, %d, %d)", res_num, freq, vol, channel);

        let me = &self.res.get_mut().mem_entries[res_num];

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

    fn snd_play_music(&mut self, res_num: u16, delay: u16, pos: u8) -> Result<()> {
        // debug(DBG_SND, "snd_play_music(0x%X, %d, %d)", res_num, delay, pos);

        if res_num != 0 {
            self.player.load_sfx_module(res_num, delay, pos)?;
            self.player.start();
        } else if delay != 0 {
            self.player.set_events_delay(delay);
        } else {
            self.player.stop();
        }

        Ok(())
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        ser.save_or_load_entries(self, Ver(1))?;

        self.video.save_or_load(ser)?;

        if ser.mode() == Mode::Load {
            self.player.stop();
        }

        self.player.save_or_load(ser)
    }

    // fn bypassProtection(&mut self)
    // {
    //     File f(true);

    //     if (!f.open("bank0e", self.res.get().getDataDir(), "rb")) {
    //         warning("Unable to bypass protection: add bank0e file to datadir");
    //     } else {
    //         Serializer s(&f, Serializer::SM_LOAD, self.res.get()._memPtrStart, 2);
    //         this->saveOrLoad(s);
    //         self.res.get().saveOrLoad(s);
    //         self.video.get().saveOrLoad(s);
    //         self.player.get().saveOrLoad(s);
    //         self.mixer.get().saveOrLoad(s);
    //     }
    //     f.close();
    // }
}

// TODO: use proc_macro

impl AccessorWrap for VirtualMachine {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.vm_variables.read(stream)?;
        self.script_stack_calls.read(stream)?;
        self.threads_data.read(stream)?;
        self.vm_is_channel_active.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.vm_variables.write(stream)?;
        self.script_stack_calls.write(stream)?;
        self.threads_data.write(stream)?;
        self.vm_is_channel_active.write(stream)
    }

    fn size(&self) -> usize {
        self.vm_variables.size()
            + self.script_stack_calls.size()
            + self.threads_data.size()
            + self.vm_is_channel_active.size()
    }
}
