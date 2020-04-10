use crate::reference::Ref;

pub const NUM_COLORS: usize = 16;
pub const BYTE_PER_PIXEL: usize = 3;

pub const DIR_LEFT: u8 = 1 << 0;
pub const DIR_RIGHT: u8 = 1 << 1;
pub const DIR_UP: u8 = 1 << 2;
pub const DIR_DOWN: u8 = 1 << 3;

pub(crate) struct PlayerInput {
    pub dir_mask: u8,
    pub button: bool,
    pub code: bool,
    pub pause: bool,
    pub quit: bool,
    pub last_char: u8,
    pub save: bool,
    pub load: bool,
    pub fast_mode: bool,
    pub state_slot: u8,
}

type AudioCallback = dyn FnMut(usize) -> Vec<u8>;
type TimerCallback = dyn FnMut(u32) -> u32;

pub(crate) type SystemRef = Ref<Box<dyn System>>;

/*
    System is an abstract class so any find of system can be plugged underneath.
*/
pub(crate) trait System {
    // typedef void (*AudioCallback)(void *param, uint8_t *stream, int len);
    // typedef uint32_t (*TimerCallback)(uint32_t delay, void *param);

    fn input(&self) -> &PlayerInput;
    fn input_mut(&mut self) -> &mut PlayerInput;

    fn init(&mut self, title: &str);
    fn destroy(&mut self);

    fn set_palette(&mut self, s: u8, n: u8, buf: &[u8]);
    fn copy_rect(&mut self, x: u16, y: u16, w: u16, h: u16, buf: &[u8], pitch: u32);

    fn process_events(&mut self);
    fn sleep(&self, duration: u32);
    fn get_timestamp(&self) -> u32;

    fn start_audio(&mut self, callback: &AudioCallback);
    fn stop_audio(&mut self);
    fn get_output_sample_rate(&mut self) -> u32;

    fn add_timer(&mut self, delay: u32, callback: &TimerCallback) -> Vec<u8>;
    fn remove_timer(&mut self, timer_id: &[u8]);

    fn create_mutex(&mut self) -> Vec<u8>;
    fn destroy_mutex(&mut self, mutex: &[u8]);
    fn lock_mutex(&mut self, mutex: &[u8]);
    fn unlock_mutex(&mut self, mutex: &[u8]);

    fn get_offscreen_framebuffer(&mut self) -> Vec<u8>;
}

pub(crate) struct MutexStack {
    sys: SystemRef,
    mutex: Vec<u8>,
}

impl MutexStack {
    pub fn new(sys: SystemRef, mutex: &[u8]) -> Self {
        let res = Self {
            sys,
            mutex: mutex.to_vec(),
        };
        res.sys.get_mut().lock_mutex(&res.mutex);
        res
    }
}

impl Drop for MutexStack {
    fn drop(&mut self) {
        self.sys.get_mut().unlock_mutex(&self.mutex);
    }
}
