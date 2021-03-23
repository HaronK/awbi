use crate::{program::Program, resource::*, serializer::*, system::*, video::Video, vm_context::*};
use anyhow::Result;

use std::{collections::HashMap, fmt};
use trace::trace;

trace::init_depth_var!();

const VM_NO_SETVEC_REQUESTED: u16 = 0xFFFF;
const VM_INACTIVE_THREAD: u16 = 0xFFFF;

pub(crate) struct VirtualMachine {
    sys: SystemRef,
    res: ResourceRef,

    data_page_idx: usize,
    data_page_offset: usize,
    stack_ptr: usize,
    goto_next_thread: bool,

    ctx: VmContext,
    programs: HashMap<usize, Program>,
    program_id: usize,
}

impl VirtualMachine {
    pub fn new(res: ResourceRef, sys: SystemRef) -> Self {
        let code_idx = res.get().seg_code_idx();
        let ctx = VmContext::new(sys.clone(), res.clone());

        Self {
            sys,
            res,

            data_page_idx: code_idx,
            data_page_offset: 0,
            stack_ptr: 0,
            goto_next_thread: false,
            ctx,
            programs: HashMap::new(),
            program_id: 0,
        }
    }

    #[trace]
    pub fn init(&mut self) {
        self.ctx.init();
    }

    #[trace]
    pub fn init_for_part(&mut self, part_id: u16) -> Result<()> {
        self.ctx.init_for_part(part_id)?;

        self.program_id = self.res.get().seg_code_idx();

        if self.programs.get(&self.program_id).is_none() {
            let mut program = Program::new(
                part_id,
                self.res.get().get_entry_data(self.program_id).into(),
            );

            program.parse()?;
            program.start();

            self.programs.insert(self.program_id, program);
        }

        Ok(())
    }

    pub fn toggle_fast_mode(&mut self) {
        self.ctx.toggle_fast_mode();
    }

    /*
         This is called every frames in the infinite loop.
    */
    #[trace]
    pub fn check_thread_requests(&mut self) -> Result<()> {
        //Check if a part switch has been requested.
        let requested_next_part = self.res.get().requested_next_part;
        if let Some(requested_next_part) = requested_next_part {
            println!("\requested_next_part={:#04x}", requested_next_part);
            self.init_for_part(requested_next_part)?;
            self.res.get_mut().requested_next_part = None;
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
            self.ctx.threads_data[thread_id].cur_state_active =
                self.ctx.threads_data[thread_id].requested_state_active;

            let n = self.ctx.threads_data[thread_id].requested_pc_offset;

            if n != VM_NO_SETVEC_REQUESTED {
                println!("\tn={:#04x}", n);
                self.ctx.threads_data[thread_id].pc_offset =
                    if n == 0xFFFE { VM_INACTIVE_THREAD } else { n };
                self.ctx.threads_data[thread_id].requested_pc_offset = VM_NO_SETVEC_REQUESTED;
            }
        }

        Ok(())
    }

    #[trace]
    pub fn host_frame(&mut self) -> Result<()> {
        // Run the Virtual Machine for every active threads (one vm frame).
        // Inactive threads are marked with a thread instruction pointer set to 0xFFFF (VM_INACTIVE_THREAD).
        // A thread must feature a break opcode so the interpreter can move to the next thread.

        for thread_id in 0..VM_NUM_THREADS {
            if !self.ctx.threads_data[thread_id].cur_state_active {
                println!("\tVirtualMachine::host_frame(skip) thread_id={}", thread_id);
                continue;
            }

            println!("TEST");

            let n = self.ctx.threads_data[thread_id].pc_offset;

            if n != VM_INACTIVE_THREAD {
                // Set the script pointer to the right location.
                // script pc is used in execute_thread in order
                // to get the next opcode.
                self.data_page_offset = n as usize;
                self.stack_ptr = 0;

                println!(
                    "\tVirtualMachine::host_frame() thread_id={} ip={}",
                    thread_id, n
                );
                self.execute_thread()?;

                //Since .pc is going to be modified by this next loop iteration, we need to save it.
                self.ctx.threads_data[thread_id].pc_offset = self.data_page_offset as u16;

                // debug(DBG_VM, "VirtualMachine::host_frame() i=0x%02X pos=0x%X", thread_id, self.threads_data[PC_OFFSET][thread_id]);
                if self.sys.get().input().quit {
                    break;
                }
            }
        }

        Ok(())
    }

    #[trace]
    fn execute_thread(&mut self) -> Result<()> {
        self.goto_next_thread = false;

        while !self.goto_next_thread {
            let opcode = 0;
            println!("\topcode=0x{:02x}", opcode);

            self.execute_opcode(opcode)?;
        }

        Ok(())
    }

    #[trace]
    fn execute_opcode(&mut self, opcode: u8) -> Result<()> {
        if let Some(program) = self.programs.get_mut(&self.program_id) {
            program.exec(&mut self.ctx)?;
        }
        Ok(())
    }

    // #[trace]
    pub fn inp_update_player(&mut self) {
        self.ctx.inp_update_player();
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        self.ctx.save_or_load(ser)
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

impl fmt::Debug for VirtualMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualMachine")
            .field("data_page_idx", &self.data_page_idx)
            .field("data_page_offset", &self.data_page_offset)
            .field("stack_ptr", &self.stack_ptr)
            .field("goto_next_thread", &self.goto_next_thread)
            .field("ctx", &self.ctx)
            .field("program_id", &self.program_id)
            .finish()
    }
}
