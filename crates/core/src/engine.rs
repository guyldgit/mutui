use std::path::Path;

use crate::error::Result;

pub(crate) trait Engine: Send {
    fn play_path(&mut self, path: &Path) -> Result<()>;
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn set_volume(&mut self, volume: f32);
    fn volume(&self) -> f32;
    fn is_paused(&self) -> bool;
    fn is_idle(&self) -> bool;
}

pub(crate) struct NullEngine {
    volume: f32,
    paused: bool,
    idle: bool,
}

impl NullEngine {
    pub(crate) fn new() -> Self {
        Self {
            volume: 1.0,
            paused: false,
            idle: true,
        }
    }
}

impl Engine for NullEngine {
    fn play_path(&mut self, _path: &Path) -> Result<()> {
        self.paused = false;
        self.idle = false;
        Ok(())
    }
    fn pause(&mut self) {
        self.paused = true;
    }
    fn resume(&mut self) {
        self.paused = false;
    }
    fn stop(&mut self) {
        self.paused = false;
        self.idle = true;
    }
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
    fn volume(&self) -> f32 {
        self.volume
    }
    fn is_paused(&self) -> bool {
        self.paused
    }
    fn is_idle(&self) -> bool {
        self.idle
    }
}
