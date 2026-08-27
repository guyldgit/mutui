use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug)]
pub enum PlayerError {
    Io(std::io::Error),
    Decode(rodio::decoder::DecoderError),
}

impl From<std::io::Error> for PlayerError {
    fn from(e: std::io::Error) -> Self {
        PlayerError::Io(e)
    }
}

impl From<rodio::decoder::DecoderError> for PlayerError {
    fn from(e: rodio::decoder::DecoderError) -> Self {
        PlayerError::Decode(e)
    }
}

pub struct Player {
    handle: OutputStreamHandle,
    sink: Sink,
    current_path: Option<PathBuf>,
    volume: f32,
    is_paused: bool,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        let (stream, handle) = OutputStreamHandle::try_default()
            .map_err(|e| PlayerError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
            )))?;

        let sink = Sink::try_new(&handle)
            .map_err(|e| PlayerError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
            )))?;

        Ok(Self {
            handle,
            sink,
            current_path: None,
            volume: 0.8,
            is_paused: false,
        })
    }

    pub fn play_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), PlayerError> {
        let path = path.as_ref();

        self.sink.stop();

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let source = Decoder::new(reader)?;

        self.sink.append(source);
        self.sink.set_volume(self.volume);
        self.sink.play();

        self.current_path = Some(path.to_path_buf());
        self.is_paused = false;

        Ok(())
    }

    pub fn pause(&mut self) {
        self.sink.pause();
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.sink.play();
        self.is_paused = false;
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_path = None;
        self.is_paused = false;
    }

    pub fn set_volume(&mut self) {
        self.volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }

    pub fn volume(&self) {
        self.is_paused
    }

    pub fn is_empty(&self) {
        self.sink.empty()
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }
}
