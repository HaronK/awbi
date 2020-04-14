use crate::file::File;
use crate::memlist::*;
use crate::parts::*;
use crate::reference::*;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use crate::vm::*;
use anyhow::{ensure, Result};

use trace::trace;

trace::init_depth_var!();

const MAX_SAVE_SLOTS: u8 = 100;
const FORMAT_SIG: u32 = 1096242006; // 'AWSV'

pub(crate) struct Engine {
    sys: SystemRef,
    vm: VirtualMachine,
    res: ResourceRef,
    data_dir: String,
    save_dir: String,
    state_slot: u8,
}

impl Engine {
    fn new(sys: SystemRef, data_dir: &str, save_dir: &str) -> Self {
        let mem_list = MemList::new(data_dir);
        let res = Ref::new(Box::new(Resource::new(mem_list)));
        let vm = VirtualMachine::new(res.clone(), sys.clone());

        Self {
            sys,
            vm,
            res,
            data_dir: data_dir.into(),
            save_dir: save_dir.into(),
            state_slot: 0,
        }
    }

    fn is_quit(&mut self) -> bool {
        self.sys.get().input().quit
    }

    #[trace]
    fn run(&mut self) -> Result<()> {
        while !self.is_quit() {
            self.vm.check_thread_requests()?;
            self.vm.inp_update_player();
            self.process_input()?;
            self.vm.host_frame()?;
        }

        Ok(())
    }

    #[trace]
    fn init(&mut self) -> Result<()> {
        //Init system
        self.sys.get_mut().init("Out Of This World");
        self.res.get_mut().reset_mem_block();
        self.res.get_mut().read_entries()?;
        self.vm.init();

        //Init virtual machine, legacy way
        self.vm.init_for_part(GAME_PART_FIRST)?; // This game part is the protection screen

        // Try to cheat here. You can jump anywhere but the VM crashes afterward.
        // Starting somewhere is probably not enough, the variables and calls return are probably missing.
        //vm.initForPart(GAME_PART2); // Skip protection screen and go directly to intro
        //vm.initForPart(GAME_PART3); // CRASH
        //vm.initForPart(GAME_PART4); // Start directly in jail but then crash
        //vm.initForPart(GAME_PART5);   //CRASH
        //vm.initForPart(GAME_PART6);   // Start in the battlechar but CRASH afteward
        //vm.initForPart(GAME_PART7); //CRASH
        //vm.initForPart(GAME_PART8); //CRASH
        //vm.initForPart(GAME_PART9); // Green screen not doing anything

        Ok(())
    }

    #[trace]
    fn process_input(&mut self) -> Result<()> {
        let mut sys = self.sys.get_mut();

        if sys.input().load {
            // self.load_game_state(self.state_slot)?; // TODO: uncomment
            sys.input_mut().load = false;
            todo!();
        }
        if sys.input().save {
            // self.save_game_state(self.state_slot, "quicksave"); // TODO: uncomment
            sys.input_mut().save = false;
            todo!();
        }
        if sys.input().fast_mode {
            self.vm.fast_mode = !self.vm.fast_mode;
            sys.input_mut().fast_mode = false;
        }
        if sys.input().state_slot != 0 {
            let slot = self.state_slot + sys.input().state_slot;
            if slot >= 0 && slot < MAX_SAVE_SLOTS {
                self.state_slot = slot;
                // debug(DBG_INFO, "Current game state slot is %d", _stateSlot);
            }
            sys.input_mut().state_slot = 0;
        }

        Ok(())
    }

    fn save_game_state(&mut self, slot: u8, desc: &str) -> Result<()> {
        let state_file = format!("raw.s{:02}", slot);

        let mut f = File::open(&state_file, &self.save_dir, false)?;
        // warning("Unable to save state file '%s'", stateFile);

        // header
        f.write_u32(FORMAT_SIG)?;
        f.write_u16(CUR_VER.0)?;
        f.write_u16(0)?;
        f.write(&desc.as_bytes()[..32])?;

        // contents
        let mut s = Serializer::new(f, Mode::Save, self.res.get().mem_buf.to_vec(), CUR_VER);
        self.vm.save_or_load(&mut s)?;
        self.res.get_mut().save_or_load(&mut s)?;

        // debug(DBG_INFO, "Saved state to slot %d", _stateSlot);

        Ok(())
    }

    fn load_game_state(&mut self, slot: u8) -> Result<()> {
        let state_file = format!("raw.s{:02}", slot);

        let mut f = File::open(&state_file, &self.save_dir, false)?;
        // warning("Unable to open state file '%s'", stateFile);

        let id = f.read_u32()?;
        ensure!(id == FORMAT_SIG, "Bad savegame format");

        // header
        let ver = f.read_u16()?;
        f.read_u16()?;

        let mut hdrdesc = [0u8; 32];
        f.read(&mut hdrdesc)?;

        // contents
        // Serializer s(&f, Serializer::SM_LOAD, res._memPtrStart, ver);
        let mut s = Serializer::new(f, Mode::Load, self.res.get().mem_buf.to_vec(), Ver(ver));
        self.vm.save_or_load(&mut s)?;
        self.res.get_mut().save_or_load(&mut s)?;

        // debug(DBG_INFO, "Loaded state from slot %d", _stateSlot);

        Ok(())
    }

    fn data_dir(&self) -> &String {
        &self.data_dir
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

    #[derive(Default)]
    struct SystemMock {
        input: PlayerInput,
    }

    impl System for SystemMock {
        fn input(&self) -> &PlayerInput {
            &self.input
        }

        fn input_mut(&mut self) -> &mut PlayerInput {
            &mut self.input
        }

        fn init(&mut self, _title: &str) {}
        fn destroy(&mut self) {}
        fn set_palette(&mut self, _s: u8, _n: u8, _buf: &[u8]) {}
        fn copy_rect(&mut self, _x: u16, _y: u16, _w: u16, _h: u16, _buf: &[u8], _pitch: u32) {}
        fn process_events(&mut self) {}
        fn sleep(&self, _duration: u32) {}

        fn get_timestamp(&self) -> u32 {
            0
        }

        fn start_audio(&mut self, _callback: &AudioCallback) {}
        fn stop_audio(&mut self) {}

        fn get_output_sample_rate(&mut self) -> u32 {
            22050 // sound sample rate
        }

        fn add_timer(&mut self, _delay: u32, _callback: &TimerCallback) -> Vec<u8> {
            vec![]
        }

        fn remove_timer(&mut self, _timer_id: &[u8]) {}

        fn create_mutex(&mut self) -> Vec<u8> {
            vec![]
        }

        fn destroy_mutex(&mut self, _mutex: &[u8]) {}
        fn lock_mutex(&mut self, _mutex: &[u8]) {}
        fn unlock_mutex(&mut self, _mutex: &[u8]) {}

        fn get_offscreen_framebuffer(&mut self) -> Vec<u8> {
            vec![]
        }
    }

    #[test]
    #[ignore]
    fn test_engine() -> Result<()> {
        let data_dir = data_dir()?;
        let sys: Ref<Box<(dyn System)>> = Ref::new(Box::new(SystemMock::default()));
        let mut engine = Engine::new(sys, data_dir.to_str().unwrap(), data_dir.to_str().unwrap());

        engine.init()?;
        engine.run()
    }
}
