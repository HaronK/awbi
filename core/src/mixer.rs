use crate::file::*;
use crate::reference::*;
use crate::serializer::*;
use crate::system::*;
use anyhow::Result;

#[derive(Default)]
pub struct MixerChunk {
    pub(crate) data: u32,
    pub(crate) len: u16,
    pub(crate) loop_pos: u16,
    pub(crate) loop_len: u16,
}

// TODO: use proc_macro

impl AccessorWrap for MixerChunk {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.data.read(stream)?;
        self.len.read(stream)?;
        self.loop_pos.read(stream)?;
        self.loop_len.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.data.write(stream)?;
        self.len.write(stream)?;
        self.loop_pos.write(stream)?;
        self.loop_len.write(stream)
    }

    fn size(&self) -> usize {
        self.data.size() + self.len.size() + self.loop_pos.size() + self.loop_len.size()
    }
}

#[derive(Default)]
struct MixerChannel {
    active: bool,
    volume: u8,
    chunk_pos: u32,
    chunk_inc: u32,
    chunk: MixerChunk,
}

impl MixerChannel {
    fn new(active: bool, volume: u8, chunk: MixerChunk, chunk_pos: u32, chunk_inc: u32) -> Self {
        Self {
            active,
            volume,
            chunk_pos,
            chunk_inc,
            chunk,
        }
    }
}

// TODO: use proc_macro

impl AccessorWrap for MixerChannel {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.active.read(stream)?;
        self.volume.read(stream)?;
        self.chunk_pos.read(stream)?;
        self.chunk_inc.read(stream)?;
        self.chunk.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.active.write(stream)?;
        self.volume.write(stream)?;
        self.chunk_pos.write(stream)?;
        self.chunk_inc.write(stream)?;
        self.chunk.write(stream)
    }

    fn size(&self) -> usize {
        self.active.size()
            + self.volume.size()
            + self.chunk_pos.size()
            + self.chunk_inc.size()
            + self.chunk.size()
    }
}

const AUDIO_NUM_CHANNELS: usize = 4;

pub type MixerRef = Ref<Box<Mixer>>;

pub(crate) struct Mixer {
    sys: SystemRef,
    mutex: Vec<u8>,

    // Since the virtual machine and SDL are running simultaneously in two different threads
    // any read or write to an elements of the sound channels MUST be synchronized with a
    // mutex.
    channels: [MixerChannel; AUDIO_NUM_CHANNELS],
}

impl Mixer {
    pub fn new(sys: SystemRef) -> Self {
        Self {
            sys,
            mutex: Vec::new(),
            channels: Default::default(),
        }
    }

    pub fn init(&mut self) {
        self.channels = Default::default();
        self.mutex = self.sys.get_mut().create_mutex();
        // self.sys.get_mut().start_audio(&|len| self.mix(len));
        todo!(); // TODO: start_audio
    }

    pub fn free(&mut self) {
        self.stop_all();
        self.sys.get_mut().stop_audio();
        self.sys.get_mut().destroy_mutex(&self.mutex);
    }

    pub fn play_channel(&mut self, channel: u8, mc: MixerChunk, freq: u16, volume: u8) {
        // debug(DBG_SND, "Mixer::playChannel(%d, %d, %d)", channel, freq, volume);
        assert!((channel as usize) < AUDIO_NUM_CHANNELS);

        // The mutex is acquired in the constructor
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);

        let ch = MixerChannel::new(
            true,
            volume,
            mc,
            0,
            ((freq as u32) << 8) / self.sys.get_mut().get_output_sample_rate(),
        );
        self.channels[channel as usize] = ch;

        //At the end of the scope the MutexStack destructor is called and the mutex is released.
    }

    pub fn stop_channel(&mut self, channel: u8) {
        // debug(DBG_SND, "Mixer::stopChannel(%d)", channel);
        assert!((channel as usize) < AUDIO_NUM_CHANNELS);

        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.channels[channel as usize].active = false;
    }

    pub fn set_channel_volume(&mut self, channel: u8, volume: u8) {
        // debug(DBG_SND, "Mixer::setChannelVolume(%d, %d)", channel, volume);
        assert!((channel as usize) < AUDIO_NUM_CHANNELS);

        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.channels[channel as usize].volume = volume;
    }

    pub fn stop_all(&mut self) {
        // debug(DBG_SND, "Mixer::stopAll()");

        let _ = MutexStack::new(self.sys.clone(), &self.mutex);
        self.channels.iter_mut().for_each(|ch| ch.active = false);
    }

    // This is SDL callback. Called in order to populate the buf with len bytes.
    // The mixer iterates through all active channels and combine all sounds.

    // Since there is no way to know when SDL will ask for a buffer fill, we need
    // to synchronize with a mutex so the channels remain stable during the execution
    // of this method.
    pub fn mix(&mut self, len: usize) -> Vec<u8> {
        let _ = MutexStack::new(self.sys.clone(), &self.mutex);

        let mut buf = vec![0i8; len];

        for ch in &mut self.channels {
            if !ch.active {
                continue;
            }

            for v in &mut buf {
                let p1 = (ch.chunk_pos >> 8) as usize;

                ch.chunk_pos += ch.chunk_inc;

                let p2 = if ch.chunk.loop_len != 0 {
                    if p1 == (ch.chunk.loop_pos + ch.chunk.loop_len - 1) as usize {
                        // debug(DBG_SND, "Looping sample on channel %d", i);
                        ch.chunk_pos = ch.chunk.loop_pos as u32;
                        ch.chunk_pos as usize
                    } else {
                        p1 + 1
                    }
                } else {
                    if p1 == ch.chunk.len as usize - 1 {
                        // debug(DBG_SND, "Stopping sample on channel %d", i);
                        ch.active = false;
                        break;
                    } else {
                        p1 + 1
                    }
                };

                // interpolate
                let b1 = get_byte(ch.chunk.data, p1) as u32;
                let b2 = get_byte(ch.chunk.data, p2) as u32;
                let ilc = ch.chunk_pos & 0xFF;
                let b = ((b1 * (0xFF - ilc) + b2 * ilc) >> 8) * (ch.volume as u32) / 0x40; //0x40=64

                // set volume and clamp
                *v = add_clamp(*v as i32, b as i32);
            }
        }

        // Convert signed 8-bit PCM to unsigned 8-bit PCM. The
        // current version of SDL hangs when using signed 8-bit
        // PCM in combination with the PulseAudio driver.
        buf.iter().map(|v| (*v as i16 + 128) as u8).collect()
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) {
        self.sys.get_mut().lock_mutex(&self.mutex);

        for ch in &mut self.channels {
            ser.save_or_load_entries(ch, Ver(2));
        }

        self.sys.get_mut().unlock_mutex(&self.mutex);
    }
}

fn get_byte(val: u32, idx: usize) -> u8 {
    val.to_ne_bytes()[idx]
}

fn add_clamp(a: i32, b: i32) -> i8 {
    let mut add = a + b;
    if add < -128 {
        add = -128;
    } else if add > 127 {
        add = 127;
    }
    return add as i8;
}
