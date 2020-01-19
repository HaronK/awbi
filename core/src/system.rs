use crate::reference::Ref;

pub const NUM_COLORS: u8 = 16;
pub const BYTE_PER_PIXEL: u8 = 3;

pub const DIR_LEFT: u8 = 1 << 0;
pub const DIR_RIGHT: u8 = 1 << 1;
pub const DIR_UP: u8 = 1 << 2;
pub const DIR_DOWN: u8 = 1 << 3;

pub struct PlayerInput {
    dir_mask: u8,
    button: bool,
    code: bool,
    pause: bool,
    quit: bool,
    last_char: char,
    save: bool,
    load: bool,
    fast_mode: bool,
    state_slot: u8,
}

type AudioCallback = fn(param: &[u8], stream: &[u8]);
type TimerCallback = fn(delay: u32, param: &[u8]);

/*
    System is an abstract class so any find of system can be plugged underneath.
*/
pub trait System {
    // typedef void (*AudioCallback)(void *param, uint8_t *stream, int len);
    // typedef uint32_t (*TimerCallback)(uint32_t delay, void *param);

    // PlayerInput input;

    fn init(&mut self, title: &str);
    fn destroy(&mut self);

    fn set_palette(&mut self, s: u8, n: u8, buf: &[u8]);
    fn copy_rect(&mut self, x: u16, y: u16, w: u16, h: u16, buf: &[u8], pitch: u32);

    fn process_events(&mut self);
    fn sleep(&self, duration: u32);
    fn get_timestamp(&self) -> u32;

    fn start_audio(&mut self, callback: &AudioCallback, param: &[u8]);
    fn stop_audio(&mut self);
    fn get_output_sample_rate(&mut self) -> u32;

    fn add_timer(&mut self, delay: u32, callback: &TimerCallback, param: &[u8]) -> Vec<u8>;
    fn remove_timer(&mut self, timer_id: &[u8]);

    fn create_mutex(&mut self) -> Vec<u8>;
    fn destroy_mutex(&mut self, mutex: &[u8]);
    fn lock_mutex(&mut self, mutex: &[u8]);
    fn unlock_mutex(&mut self, mutex: &[u8]);

    fn get_offscreen_framebuffer(&mut self) -> Vec<u8>;
}

pub struct MutexStack {
    sys: Ref<Box<dyn System>>,
    mutex: Vec<u8>,
}

impl MutexStack {
    pub fn new(sys: Ref<Box<dyn System>>, mutex: &[u8]) -> Self {
        let mut res = Self {
            sys,
            mutex: mutex.to_vec(),
        };
        res.sys.lock_mutex(&res.mutex);
        res
    }
}

impl Drop for MutexStack {
    fn drop(&mut self) {
        self.sys.unlock_mutex(&self.mutex);
    }
}
