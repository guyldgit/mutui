use std::fs::File;
use std::path::Path;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player as AudioOut};

use crate::engine::Engine;
use crate::error::{Error, Result};

pub(crate) struct AudioEngine {
    _sink: MixerDeviceSink,
    out: AudioOut,
    volume: f32,
}

impl AudioEngine {
    pub(crate) fn open_default() -> Result<Self> {
        let mut sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| Error::Device(e.to_string()))?;
        sink.log_on_drop(false);
        let out = AudioOut::connect_new(sink.mixer());
        Ok(Self {
            _sink: sink,
            out,
            volume: 1.0,
        })
    }
}

impl Engine for AudioEngine {
    fn play_path(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path)?;
        let decoder = Decoder::try_from(file).map_err(|e| Error::Decode(e.to_string()))?;
        self.out.stop();
        self.out.append(decoder);
        self.out.set_volume(self.volume);
        self.out.play();
        Ok(())
    }

    fn pause(&mut self) {
        self.out.pause();
    }

    fn resume(&mut self) {
        self.out.play();
    }

    fn stop(&mut self) {
        self.out.stop();
    }

    fn set_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.volume = volume;
        self.out.set_volume(volume);
    }

    fn volume(&self) -> f32 {
        self.volume
    }

    fn is_paused(&self) -> bool {
        self.out.is_paused()
    }

    fn is_idle(&self) -> bool {
        !self.is_paused() && self.out.empty()
    }
}
