use crate::file::File;
use crate::system::*;
use crate::vm::*;
use crate::mixer::*;
use crate::parts::*;
use crate::resource::*;
use crate::sfxplayer::*;
use crate::serializer::*;
use crate::video::*;
use crate::reference::*;
use anyhow::{ensure, Result};

const MAX_SAVE_SLOTS: u8 = 100;
const FORMAT_SIG: u32 = 1096242006; // 'AWSV'

pub(crate) struct Engine {
	sys: SystemRef,
	vm: VirtualMachineRef,
	mixer: MixerRef,
	res: ResourceRef,
	player: SfxPlayerRef,
	video: VideoRef,
    data_dir: String,
    save_dir: String,
	state_slot: u8,
}

impl Engine {
    fn new(sys: SystemRef, data_dir: &str, save_dir: &str) -> Self {
        let mixer = Ref::new(Box::new(Mixer::new(sys.clone())));
        let res = Ref::new(Box::new(Resource::new(data_dir.into())));
        let video = Ref::new(Box::new(Video::new(res.clone(), sys.clone())));
        let player = Ref::new(Box::new(SfxPlayer::new(mixer.clone(), res.clone(), sys.clone())));
        let vm = Ref::new(Box::new(VirtualMachine::new(mixer.clone(), res.clone(), player.clone(), video.clone(), sys.clone())));

        Self {
            sys,
            vm,
            mixer,
            res,
            player,
            video,
            data_dir: data_dir.into(),
            save_dir: save_dir.into(),
            state_slot: 0,
        }
    }

    fn run(&mut self) -> Result<()> {
        let sys = self.sys.get();
        let mut vm = self.vm.get_mut();

        while !sys.input().quit {
            vm.check_thread_requests()?;
            vm.inp_update_player();
            // self.process_input()?; // TODO: uncomment
            vm.host_frame();
        }

        Ok(())
    }

    fn init(&mut self) -> Result<()> {
        //Init system
        self.sys.get_mut().init("Out Of This World");
        self.video.get_mut().init();
        // self.res.get_mut().allocMemBlock();
        self.res.get_mut().read_entries()?;
        self.vm.get_mut().init();
        self.mixer.get_mut().init();
        self.player.get_mut().init();

        //Init virtual machine, legacy way
        self.vm.get_mut().init_for_part(GAME_PART_FIRST)?; // This game part is the protection screen

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

    fn process_input(&mut self) -> Result<()> {
        let mut sys = self.sys.get_mut();

        if sys.input().load {
            self.load_game_state(self.state_slot)?;
            sys.input_mut().load = false;
        }
        if sys.input().save {
            // self.save_game_state(self.state_slot, "quicksave"); // TODO: uncomment
            sys.input_mut().save = false;
        }
        if sys.input().fast_mode {
            let mut vm = self.vm.get_mut();

            vm.fast_mode = !vm.fast_mode;
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

    // fn make_game_state_name(slot: u8) -> String {
    //     format!("raw.s{:0>2}", slot)
    // }

    fn save_game_state(&mut self, slot: u8, desc: &str) -> Result<()> {
        let state_file = format!("raw.s{:0>2}", slot);

        let mut f = File::open(&state_file, &self.save_dir, false)?;
        // warning("Unable to save state file '%s'", stateFile);

        // header
        f.write_u32(FORMAT_SIG)?;
        f.write_u16(CUR_VER.0)?;
        f.write_u16(0)?;
        f.write(&desc.as_bytes()[..32])?;

        // contents
        let mut s = Serializer::new(f, Mode::Save, self.res.get().mem_buf.to_vec(), CUR_VER);
        self.vm.get_mut().save_or_load(&mut s)?;
        self.res.get_mut().save_or_load(&mut s)?;
        self.video.get_mut().save_or_load(&mut s)?;
        self.player.get_mut().save_or_load(&mut s)?;
        self.mixer.get_mut().save_or_load(&mut s)?;

        // debug(DBG_INFO, "Saved state to slot %d", _stateSlot);

        Ok(())
    }

    fn load_game_state(&self, slot: u8) -> Result<()> {
        let state_file = format!("raw.s{:0>2}", slot);

        let mut f = File::open(&state_file, &self.save_dir, false)?;
        // warning("Unable to open state file '%s'", stateFile);

        let id = f.read_u32()?;
        ensure!(id == FORMAT_SIG, "Bad savegame format");

        // mute
        self.player.get_mut().stop();
        self.mixer.get_mut().stop_all();

        // header
        let ver = f.read_u16()?;
        f.read_u16()?;

        let mut hdrdesc = [0u8; 32];
        f.read(&mut hdrdesc)?;

        // contents
        // Serializer s(&f, Serializer::SM_LOAD, res._memPtrStart, ver);
        let mut s = Serializer::new(f, Mode::Load, self.res.get().mem_buf.to_vec(), Ver(ver));
        self.vm.get_mut().save_or_load(&mut s)?;
        self.res.get_mut().save_or_load(&mut s)?;
        self.video.get_mut().save_or_load(&mut s)?;
        self.player.get_mut().save_or_load(&mut s)?;
        self.mixer.get_mut().save_or_load(&mut s)?;

        // debug(DBG_INFO, "Loaded state from slot %d", _stateSlot);

        Ok(())
    }

    fn data_dir(&self) -> String {
        self.data_dir.clone()
    }
}
