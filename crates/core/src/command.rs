use std::path::PathBuf;

use crate::track::Track;

#[derive(Debug, Clone)]
pub enum Command {
    PlayPaths(Vec<PathBuf>),
    Enqueue(Vec<PathBuf>),
    Pause,
    Resume,
    TogglePause,
    Stop,
    Next,
    Prev,
    SetVolume(f32),
    QueueClear,
    QueueRemove { index: usize },
    SetRepeat(Repeat),
    ToggleShuffle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub state: PlayState,
    pub volume: f32,
    pub current: Option<Track>,
    pub position_secs: Option<f64>,
    pub queue: Vec<Track>,
    pub repeat: Repeat,
    pub shuffle: bool,
}

impl Status {
    pub fn empty() -> Self {
        Self {
            state: PlayState::Stopped,
            volume: 1.0,
            current: None,
            position_secs: None,
            queue: Vec::new(),
            repeat: Repeat::Off,
            shuffle: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Started(Track),
    Ended,
    QueueChanged,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Off,
    One,
    All,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::One,
            Repeat::One => Repeat::All,
            Repeat::All => Repeat::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Repeat::Off => "off",
            Repeat::One => "one",
            Repeat::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Track;

    #[test]
    fn status_is_clone() {
        let mut s = Status::empty();
        s.queue.push(Track::from_path("/a.flac").unwrap());
        s.current = s.queue.first().cloned();
        let c = s.clone();
        assert_eq!(c.queue.len(), 1);
        assert_eq!(c.state, PlayState::Stopped);
        assert!(c.position_secs.is_none());
    }

    #[test]
    fn playpaths_is_replace_not_enqueue() {
        let cmd = Command::PlayPaths(vec!["/a.flac".into(), "/b.flac".into()]);
        match cmd {
            Command::PlayPaths(p) => assert_eq!(p.len(), 2),
            _ => panic!("wrong variant"),
        }
    }
}
