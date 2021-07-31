use anyhow::{Error, Result};
use awbi_core::system::{PlayerInput, System, TimerId, *};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Mod},
    pixels::{PixelFormatEnum, PixelMasks},
    surface::Surface,
    Sdl,
};

const ScreenWidth: u32 = 320;
const ScreenHeight: u32 = 200;
const SoundSampleRate: u16 = 22050;

type ScaleProc = fn(&mut [u16], u16, &[u16], u16, u16, u16);

struct Scaler {
    name: &'static str,
    proc: ScaleProc,
    factor: u32,
}

const Scalers: [Scaler; 5] = [
    Scaler {
        name: "Point1_tx",
        proc: point1_tx,
        factor: 1,
    },
    Scaler {
        name: "Point2_tx",
        proc: point2_tx,
        factor: 2,
    },
    Scaler {
        name: "Scale2x",
        proc: scale2x,
        factor: 2,
    },
    Scaler {
        name: "Point3_tx",
        proc: point3_tx,
        factor: 3,
    },
    Scaler {
        name: "Scale3x",
        proc: scale3x,
        factor: 3,
    },
];

const offscreen_size: usize = (ScreenWidth * ScreenHeight * 2) as usize;
pub struct SdlSystem {
    context: Sdl,
    offscreen: [u8; offscreen_size],
    fullscreen: bool,
    scaler: u8,

    input: PlayerInput,
}

impl SdlSystem {
    pub fn new() -> Result<Self> {
        Ok(Self {
            context: sdl2::init().map_err(Error::msg)?,
            offscreen: [0; offscreen_size],
            fullscreen: false,
            scaler: 1,
            input: Default::default(),
        })
    }

    fn prepare_gfx_mode(&mut self) -> Result<()> {
        let w = ScreenWidth * Scalers[self.scaler as usize].factor;
        let h = ScreenHeight * Scalers[self.scaler as usize].factor;
        let pixel_masks = PixelFormatEnum::RGBA4444.into_masks().map_err(Error::msg)?;
        let surface = Surface::from_pixelmasks(w, h, pixel_masks).map_err(Error::msg)?;

        // _screen = SDL_SetVideoMode(w, h, 16, _fullscreen ? (SDL_FULLSCREEN | SDL_HWSURFACE) : SDL_HWSURFACE);

        // if (!_screen) {
        //     error("SDLStub::prepareGfxMode() unable to allocate _screen buffer");
        // }
        // _sclscreen = SDL_CreateRGBSurface(SDL_SWSURFACE, w, h, 16,
        //                     _screen->format->Rmask,
        //                     _screen->format->Gmask,
        //                     _screen->format->Bmask,
        //                     _screen->format->Amask);
        // if (!_sclscreen) {
        //     error("SDLStub::prepareGfxMode() unable to allocate _sclscreen buffer");
        // }

        Ok(())
    }

    fn cleanup_gfx_mode(&mut self) {
        // if (_offscreen) {
        //     free(_offscreen);
        //     _offscreen = 0;
        // }
        // if (_sclscreen) {
        //     SDL_FreeSurface(_sclscreen);
        //     _sclscreen = 0;
        // }
        // if (_screen) {
        //     SDL_FreeSurface(_screen);
        //     _screen = 0;
        // }
    }

    fn switch_gfx_mode(&mut self, fullscreen: bool, scaler: u8) {
        // SDL_Surface * prev_sclscreen = _sclscreen;
        // SDL_FreeSurface(_screen);
        // _fullscreen = fullscreen;
        // _scaler = scaler;
        // prepareGfxMode();
        // SDL_BlitSurface(prev_sclscreen, NULL, _sclscreen, NULL);
        // SDL_FreeSurface(prev_sclscreen);
    }
}

impl System for SdlSystem {
    fn input(&self) -> &awbi_core::system::PlayerInput {
        &self.input
    }

    fn input_mut(&mut self) -> &mut awbi_core::system::PlayerInput {
        &mut self.input
    }

    fn init(&mut self, title: &str) -> Result<()> {
        // self.context.mouse().show_cursor(false);

        let video = self.context.video().map_err(Error::msg)?;

        let window = video
            .window(title, ScreenWidth, ScreenHeight)
            .position_centered()
            .opengl()
            .build()
            .map_err(Error::msg)?;

        self.fullscreen = false;
        self.scaler = 1;

        self.prepare_gfx_mode();

        Ok(())
    }

    fn destroy(&mut self) {
        self.cleanup_gfx_mode();
    }

    fn set_palette(&mut self, s: u8, n: u8, buf: &[u8]) {}

    fn copy_rect(&mut self, x: u16, y: u16, w: u16, h: u16, buf: &[u8], pitch: u32) {}

    fn process_events(&mut self) -> Result<()> {
        let mut event_pump = self.context.event_pump().map_err(Error::msg)?;

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => {
                        self.input.quit = true;
                        break 'running;
                    }
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Left => self.input.dir_mask &= DIR_LEFT,
                        Keycode::Right => self.input.dir_mask &= DIR_RIGHT,
                        Keycode::Up => self.input.dir_mask &= DIR_UP,
                        Keycode::Down => self.input.dir_mask &= DIR_DOWN,
                        Keycode::Space | Keycode::Return => self.input.button = false,
                        _ => {}
                    },
                    Event::KeyDown {
                        keycode: Some(keycode),
                        keymod,
                        ..
                    } => {
                        match keymod {
                            Mod::LALTMOD | Mod::RALTMOD => match keycode {
                                Keycode::Return => {
                                    self.switch_gfx_mode(!self.fullscreen, self.scaler)
                                }
                                Keycode::KpPlus => {
                                    self.switch_gfx_mode(self.fullscreen, self.scaler + 1)
                                }
                                Keycode::KpMinus => {
                                    self.switch_gfx_mode(self.fullscreen, self.scaler - 1)
                                }
                                Keycode::X => {
                                    self.input.quit = true;
                                    break 'running;
                                }
                                _ => {}
                            },
                            Mod::LCTRLMOD | Mod::RCTRLMOD => match keycode {
                                Keycode::S => self.input.save = true,
                                Keycode::L => self.input.load = true,
                                Keycode::F => self.input.fast_mode = true,
                                Keycode::KpPlus => self.input.state_slot = 1,
                                Keycode::KpMinus => self.input.state_slot = -1,
                                _ => {}
                            },
                            _ => {}
                        }

                        match event {
                            Event::KeyDown {
                                keycode: Some(keycode),
                                ..
                            } => {
                                self.input.last_char = keycode as u8;
                                match keycode {
                                    Keycode::Left => self.input.dir_mask |= DIR_LEFT,
                                    Keycode::Right => self.input.dir_mask |= DIR_RIGHT,
                                    Keycode::Up => self.input.dir_mask |= DIR_UP,
                                    Keycode::Down => self.input.dir_mask |= DIR_DOWN,
                                    Keycode::Space | Keycode::Return => self.input.button = true,
                                    Keycode::C => self.input.code = true,
                                    Keycode::P => self.input.pause = true,
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

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

fn point1_tx(dst: &mut [u16], dst_pitch: u16, src: &[u16], src_pitch: u16, w: u16, mut h: u16) {
    let dst_pitch: usize = (dst_pitch >> 1) as usize;
    let src_pitch: usize = src_pitch as usize;
    let l = (w * 2) as usize;
    let mut dst_idx = 0;
    let mut src_idx = 0;

    while h > 0 {
        dst[dst_idx..][..l].clone_from_slice(&src[src_idx..][..l]);
        dst_idx += dst_pitch;
        src_idx += src_pitch;
        h -= 1;
    }
}

fn point2_tx(dst: &mut [u16], dst_pitch: u16, src: &[u16], src_pitch: u16, w: u16, mut h: u16) {
    let dst_pitch: usize = (dst_pitch >> 1) as usize;
    let src_pitch: usize = src_pitch as usize;
    let mut dst_idx = 0;
    let mut src_idx = 0;

    while h > 0 {
        for i in 0..w as usize {
            let c = src[src_idx + i];
            dst[dst_idx] = c;
            dst[dst_idx + 1] = c;
            dst[dst_idx + dst_pitch] = c;
            dst[dst_idx + 1 + dst_pitch] = c;
            dst_idx += 2;
        }
        dst_idx += dst_pitch * 2;
        src_idx += src_pitch;
        h -= 1;
    }
}

fn point3_tx(dst: &mut [u16], dst_pitch: u16, src: &[u16], src_pitch: u16, w: u16, mut h: u16) {
    let mut dst_pitch: usize = (dst_pitch >> 1) as usize;
    let src_pitch: usize = src_pitch as usize;
    let mut dst_idx = 0;
    let mut src_idx = 0;

    while h > 0 {
        for i in 0..w as usize {
            let c = src[src_idx + i];
            dst[dst_idx] = c;
            dst[dst_idx + 1] = c;
            dst[dst_idx + 2] = c;
            dst[dst_idx + dst_pitch] = c;
            dst[dst_idx + 1 + dst_pitch] = c;
            dst[dst_idx + 2 + dst_pitch] = c;
            dst[dst_idx + dst_pitch * 2] = c;
            dst[dst_idx + 1 + dst_pitch * 2] = c;
            dst[dst_idx + 2 + dst_pitch * 2] = c;
            dst_idx += 3;
        }
        dst_idx += dst_pitch * 3;
        src_idx += src_pitch;
        h -= 1;
    }
}

fn scale2x(dst: &mut [u16], dst_pitch: u16, src: &[u16], src_pitch: u16, w: u16, mut h: u16) {
    let dst_pitch: usize = (dst_pitch >> 1) as usize;
    let src_pitch: usize = src_pitch as usize;
    let mut dst_idx = 0;
    let mut src_idx = 0;

    while h > 0 {
        for i in 0..w as usize {
            let b = src[src_idx + i - src_pitch];
            let d = src[src_idx + i - 1];
            let e = src[src_idx + i];
            let f = src[src_idx + i + 1];
            let h = src[src_idx + i + src_pitch];

            if b != h && d != f {
                dst[dst_idx] = if d == b { d } else { e };
                dst[dst_idx + 1] = if b == f { f } else { e };
                dst[dst_idx + dst_pitch] = if d == h { d } else { e };
                dst[dst_idx + dst_pitch + 1] = if h == f { f } else { e };
            } else {
                dst[dst_idx] = e;
                dst[dst_idx + 1] = e;
                dst[dst_idx + dst_pitch] = e;
                dst[dst_idx + dst_pitch + 1] = e;
            }
        }
        dst_idx += dst_pitch * 2;
        src_idx += src_pitch;
        h -= 1;
    }
}

fn scale3x(dst: &mut [u16], dst_pitch: u16, src: &[u16], src_pitch: u16, w: u16, mut h: u16) {
    let dst_pitch: usize = (dst_pitch >> 1) as usize;
    let src_pitch: usize = src_pitch as usize;
    let mut dst_idx = 0;
    let mut src_idx = 0;

    while h > 0 {
        for j in 0..w as usize {
            let a = src[src_idx + j - src_pitch - 1];
            let b = src[src_idx + j - src_pitch];
            let c = src[src_idx + j - src_pitch + 1];
            let d = src[src_idx + j - 1];
            let e = src[src_idx + j];
            let f = src[src_idx + j + 1];
            let g = src[src_idx + j + src_pitch - 1];
            let h = src[src_idx + j + src_pitch];
            let i = src[src_idx + j + src_pitch + 1];

            if b != h && d != f {
                dst[dst_idx] = if d == b { d } else { e };
                dst[dst_idx + 1] = if d == b && e != c || b == f && e != a {
                    b
                } else {
                    e
                };
                dst[dst_idx + 2] = if b == f { f } else { e };
                dst[dst_idx + dst_pitch] = if d == b && e != g || d == b && e != a {
                    d
                } else {
                    e
                };
                dst[dst_idx + dst_pitch + 1] = e;
                dst[dst_idx + dst_pitch + 2] = if b == f && e != i || h == f && e != c {
                    f
                } else {
                    e
                };
                dst[dst_idx + dst_pitch * 2] = if d == h { d } else { e };
                dst[dst_idx + dst_pitch * 2 + 1] = if d == h && e != i || h == f && e != g {
                    h
                } else {
                    e
                };
                dst[dst_idx + dst_pitch * 2 + 2] = if h == f { f } else { e };
            } else {
                dst[dst_idx] = e;
                dst[dst_idx + 1] = e;
                dst[dst_idx + 2] = e;
                dst[dst_idx + dst_pitch] = e;
                dst[dst_idx + dst_pitch + 1] = e;
                dst[dst_idx + dst_pitch + 2] = e;
                dst[dst_idx + dst_pitch * 2] = e;
                dst[dst_idx + dst_pitch * 2 + 1] = e;
                dst[dst_idx + dst_pitch * 2 + 2] = e;
            }
        }
        dst_idx += dst_pitch * 3;
        src_idx += src_pitch;
        h -= 1;
    }
}
