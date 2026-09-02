use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::command::{Command, PlayState, Status, Repeat};
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::queue::Queue;
use crate::track::Track;

use rand::Rng;

pub(crate) struct Worker {
    queue: Queue,
    engine: Box<dyn Engine>,
    state: PlayState,
    snapshot: Arc<Mutex<Status>>,
    repeat: Repeat,
    shuffle: bool,
    played: Vec<usize>,
    started_at: Option<std::time::Instant>,
}

impl Worker {
    pub(crate) fn new(engine: Box<dyn Engine>, snapshot: Arc<Mutex<Status>>) -> Self {
        let w = Self {
            queue: Queue::new(),
            engine,
            state: PlayState::Stopped,
            snapshot,
            repeat: Repeat::Off,
            shuffle: false,
            played: Vec::new(),
            started_at: None,
        };
        w.publish();
        w
    }

    pub(crate) fn run(mut self, cmds: Receiver<Command>) {
        loop {
            match cmds.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => {
                    if let Err(_e) = self.handle(cmd) {
                        self.state = PlayState::Stopped;
                        self.engine.stop();
                    }
                    self.publish();
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.on_possible_end() {
                        self.publish();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::PlayPaths(paths) => self.play_paths(paths),
            Command::Enqueue(paths) => self.enqueue(paths),
            Command::Pause => {
                self.engine.pause();
                if self.state == PlayState::Playing {
                    self.state = PlayState::Paused;
                }
                Ok(())
            }
            Command::Resume => {
                if self.state == PlayState::Paused {
                    self.engine.resume();
                    self.state = PlayState::Playing;
                }
                Ok(())
            }
            Command::TogglePause => {
                match self.state {
                    PlayState::Playing => self.handle(Command::Pause),
                    PlayState::Paused => self.handle(Command::Resume),
                    PlayState::Stopped => {
                        if let Ok(t) = self.queue.current() {
                            let path = t.path.clone();
                            self.engine.play_path(&path)?;
                            self.state = PlayState::Playing;
                        }
                        Ok(())
                    }
                }
            }
            Command::Stop => {
                self.engine.stop();
                self.state = PlayState::Stopped;
                Ok(())
            }
            Command::Next => self.advance(true),
            Command::Prev => self.retreat(),
            Command::SetVolume(v) => {
                self.engine.set_volume(v);
                Ok(())
            }
            Command::QueueClear => {
                self.queue.clear();
                self.engine.stop();
                self.state = PlayState::Stopped;
                Ok(())
            }
            Command::QueueRemove { index } => {
                let removed_current = self.queue.current_index() == Some(index);
                self.queue.remove(index)?;
                if removed_current {
                    self.restart_current_or_stop()?;
                }
                Ok(())
            }
            Command::SetRepeat(r) => {
                self.repeat = r;
                Ok(())
            }
            Command::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                self.played.clear();
                Ok(())
            }
            Command::PlayIndex(i) => self.play_index(i),
        }
    }

    fn play_paths(&mut self, paths: Vec<std::path::PathBuf>) -> Result<()> {
        self.queue.clear();
        if paths.is_empty() {
            self.engine.stop();
            self.state = PlayState::Stopped;
            return Ok(());
        }
        for p in paths {
            self.queue.push(Track::from_path(p)?);
        }
        self.restart_current_or_stop()
    }

    fn enqueue(&mut self, paths: Vec<std::path::PathBuf>) -> Result<()> {
        let was_empty = self.queue.is_empty();
        for p in paths {
            self.queue.push(Track::from_path(p)?);
        }
        if was_empty && !self.queue.is_empty() {
            self.restart_current_or_stop()?;
        }
        Ok(())
    }

    fn restart_current_or_stop(&mut self) -> Result<()> {
        match self.queue.current() {
            Ok(t) => {
                let path = t.path.clone();
                self.engine.play_path(&path)?;
                self.state = PlayState::Playing;
                self.started_at = Some(std::time::Instant::now());
                Ok(())
            }
            Err(_) => {
                self.engine.stop();
                self.state = PlayState::Stopped;
                self.started_at = None;
                Ok(())
            }
        }
    }

    fn on_possible_end(&mut self) -> bool {
        if let Some(t) = self.started_at {
            if t.elapsed() < Duration::from_millis(300) {
                return false;
            }
        }
        if self.state != PlayState::Playing || !self.engine.is_idle() {
            return false;
        }
        let _ = self.advance(false);
        true
    }

    fn publish(&self) {
        let status = Status {
            state: self.state,
            volume: self.engine.volume(),
            current: self.queue.current().ok().cloned(),
            position_secs: None,
            queue: self.queue.tracks().to_vec(),
            repeat: self.repeat,
            shuffle: self.shuffle,
        };
        if let Ok(mut slot) = self.snapshot.lock() {
            *slot = status;
        }
    }

    fn play_index(&mut self, i: usize) -> Result<()> {
        self.queue.set_current(i)?;
        self.restart_current_or_stop()
    }

    fn advance(&mut self, from_user: bool) -> Result<()> {
        if self.queue.is_empty() {
            return Err(Error::QueueEmpty);
        }

        if !from_user && self.repeat == Repeat::One {
            return self.restart_current_or_stop();
        }

        if self.shuffle {
            return self.advance_shuffle(from_user);
        }

        let i = self.queue.current_index().unwrap_or(0);
        let last = self.queue.len() - 1;

        let next = if from_user {
            if i >= last { 0 } else { i + 1 }
        } else if i >= last {
            if self.repeat == Repeat::All {
                0
            } else {
                self.engine.stop();
                self.state = PlayState::Stopped;
                return Ok(());
            }
        } else {
            i + 1
        };

        self.play_index(next)
    }

    fn retreat(&mut self) -> Result<()> {
        if self.queue.is_empty() {
            return Err(Error::QueueEmpty);
        }
        if self.shuffle {
            return self.advance_shuffle(true);
        }
        let i = self.queue.current_index().unwrap_or(0);
        let next = if i == 0 { self.queue.len() - 1 } else { i - 1 };
        self.play_index(next)
    }

    fn advance_shuffle(&mut self, from_user: bool) -> Result<()> {
        let n = self.queue.len();
        if let Some(i) = self.queue.current_index() {
            if !self.played.contains(&i) {
                self.played.push(i);
            }
        }
        let current = self.queue.current_index();
        let candidates: Vec<usize> = (0..n)
            .filter(|i| Some(*i) != current && !self.played.contains(i))
            .collect();

        if candidates.is_empty() {
            if self.repeat == Repeat::All || from_user {
                self.played.clear();
                let i = 0;
                return self.play_index(i);
            }
            self.engine.stop();
            self.state = PlayState::Stopped;
            return Ok(());
        }
        let i = candidates[rand::thread_rng().gen_range(0..candidates.len())];
        self.play_index(i)
    }
}
