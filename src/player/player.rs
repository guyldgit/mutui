use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player as AudioOut, Source};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug)]
pub enum PlayerError {
    Io(std::io::Error),
    Decode(String),
    Stream(String),
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerError::Io(e) => write!(f, "io error: {e}"),
            PlayerError::Decode(e) => write!(f, "decode error: {e}"),
            PlayerError::Stream(e) => write!(f, "audio stream error: {e}"),
        }
    }
}

impl std::error::Error for PlayerError {}

impl From<std::io::Error> for PlayerError {
    fn from(e: std::io::Error) -> Self {
        PlayerError::Io(e)
    }
}

pub struct Player {
    _stream: MixerDeviceSink,
    out: AudioOut,
    current_path: Option<PathBuf>,
    duration: Option<Duration>,
    volume: f32,
    is_paused: bool,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| PlayerError::Stream(e.to_string()))?;

        let out = AudioOut::connect_new(stream.mixer());

        Ok(Self {
            _stream: stream,
            out,
            current_path: None,
            duration: None,
            volume: 0.8,
            is_paused: false,
        })
    }

    pub fn play_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), PlayerError> {
        let path = path.as_ref();
        self.out.stop();

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).map_err(|e| PlayerError::Decode(e.to_string()))?;

        self.duration = source.total_duration();
        self.out.append(source);
        self.out.set_volume(self.volume);
        self.out.play();

        self.current_path = Some(path.to_path_buf());
        self.is_paused = false;
        Ok(())
    }

    pub fn pause(&mut self) {
        self.out.pause();
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.out.play();
        self.is_paused = false;
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        self.out.stop();
        self.current_path = None;
        self.is_paused = false;
        self.duration = None;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.out.set_volume(self.volume);
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn is_empty(&self) -> bool {
        self.out.empty()
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    pub fn position(&self) -> Duration {
        if self.is_empty() {
            Duration::ZERO
        } else {
            self.out.get_pos()
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn progress(&self) -> f64 {
        match self.duration {
            Some(total) if total.as_secs_f64() > 0.0 => {
                (self.position().as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }
}
