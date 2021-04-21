use awbi_core::system::{PlayerInput, System, TimerId};

#[derive(Default)]
pub struct SdlSystem {
    input: PlayerInput,
}

impl System for SdlSystem {
    fn input(&self) -> &awbi_core::system::PlayerInput {
        &self.input
    }

    fn input_mut(&mut self) -> &mut awbi_core::system::PlayerInput {
        &mut self.input
    }

    fn init(&mut self, title: &str) {}

    fn destroy(&mut self) {}

    fn set_palette(&mut self, s: u8, n: u8, buf: &[u8]) {}

    fn copy_rect(&mut self, x: u16, y: u16, w: u16, h: u16, buf: &[u8], pitch: u32) {}

    fn process_events(&mut self) {}

    fn sleep(&self, duration: u32) {}

    fn get_timestamp(&self) -> u32 {
        0
    }

    fn start_audio(&mut self, callback: &awbi_core::system::AudioCallback) {}

    fn stop_audio(&mut self) {}

    fn get_output_sample_rate(&mut self) -> u32 {
        22050 // sound sample rate
    }

    fn add_timer(
        &mut self,
        delay: u32,
        callback: &awbi_core::system::TimerCallback,
    ) -> awbi_core::system::TimerId {
        TimerId::default()
    }

    fn remove_timer(&mut self, timer_id: awbi_core::system::TimerId) {}

    fn create_mutex(&mut self) -> Vec<u8> {
        vec![]
    }

    fn destroy_mutex(&mut self, mutex: &[u8]) {}

    fn lock_mutex(&mut self, mutex: &[u8]) {}

    fn unlock_mutex(&mut self, mutex: &[u8]) {}

    fn get_offscreen_framebuffer(&mut self) -> Vec<u8> {
        vec![]
    }
}
