use std::path::Path;

use crate::audio::AudioEngine;
use crate::engine::Engine;
use crate::error::Result;

pub struct Player {
    engine: AudioEngine,
}

impl Player {
    pub fn new() -> Result<Self> {
        Ok(Self {
            engine: AudioEngine::open_default()?,
        })
    }

    pub fn play_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.engine.play_path(path.as_ref())
    }

    pub fn pause(&mut self) {
        self.engine.pause();
    }

    pub fn resume(&mut self) {
        self.engine.resume();
    }

    pub fn stop(&mut self) {
        self.engine.stop();
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.engine.set_volume(volume);
    }

    pub fn volume(&self) -> f32 {
        self.engine.volume()
    }

    pub fn is_paused(&self) -> bool {
        self.engine.is_paused()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_still_device_free() {
        let mut q = crate::Queue::new();
        q.push(crate::Track::from_path("/a.flac").unwrap());
        assert_eq!(q.len(), 1);
    }

    #[test]
    #[ignore = "needs a real output device"]
    fn opens_default_device() {
        let p = Player::new().expect("device");
        assert!(p.volume() > 0.0 || p.volume() == 0.0);
    }
}
