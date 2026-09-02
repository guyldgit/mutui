use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::audio::AudioEngine;
use crate::command::{Command, Status};
use crate::engine::{Engine, NullEngine};
use crate::error::{Error, Result};
use crate::worker::Worker;

#[derive(Clone)]
pub struct Handle {
    cmds: Sender<Command>,
    snapshot: Arc<Mutex<Status>>,
    _worker: Option<Arc<WorkerGuard>>,
}

struct WorkerGuard {
    thread: Option<JoinHandle<()>>,
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Handle {
    pub fn start() -> Result<Self> {
        let engine = AudioEngine::open_default()?;
        Self::spawn(Box::new(engine))
    }

    pub fn start_null() -> Self {
        Self::spawn(Box::new(NullEngine::new())).expect("null engine cannot fail")
    }

    fn spawn(engine: Box<dyn Engine>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Status::empty()));
        let snap2 = Arc::clone(&snapshot);
        let thread = thread::Builder::new()
            .name("mutui-worker".into())
            .spawn(move || Worker::new(engine, snap2).run(rx))
            .map_err(|e| Error::Device(e.to_string()))?;

        Ok(Self {
            cmds: tx,
            snapshot,
            _worker: Some(Arc::new(WorkerGuard {
                thread: Some(thread),
            })),
        })
    }

    pub fn send(&self, cmd: Command) -> Result<()> {
        self.cmds.send(cmd).map_err(|_| Error::ChannelClosed)
    }

    pub fn status(&self) -> Status {
        self.snapshot
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, PlayState};
    use std::thread;
    use std::time::Duration;

    fn wait_playing(h: &Handle) {
        for _ in 0..20 {
            if h.status().state == PlayState::Playing {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("never started: {:?}", h.status());
    }

    #[test]
    fn null_enqueue_pause_status() {
        let h = Handle::start_null();
        h.send(Command::Enqueue(vec!["/tmp/a.flac".into()]))
            .unwrap();
        wait_playing(&h);
        let s = h.status();
        assert_eq!(s.queue.len(), 1);
        assert_eq!(s.current.as_ref().unwrap().title, "a");

        h.send(Command::Pause).unwrap();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(h.status().state, PlayState::Paused);

        h.send(Command::SetVolume(0.25)).unwrap();
        thread::sleep(Duration::from_millis(30));
        assert!((h.status().volume - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn playpaths_replaces() {
        let h = Handle::start_null();
        h.send(Command::Enqueue(vec!["/a.flac".into()])).unwrap();
        wait_playing(&h);
        h.send(Command::PlayPaths(vec!["/b.flac".into(), "/c.flac".into()]))
            .unwrap();
        thread::sleep(Duration::from_millis(30));
        let s = h.status();
        assert_eq!(s.queue.len(), 2);
        assert_eq!(s.current.as_ref().unwrap().title, "b");
    }

    #[test]
    fn enqueue_does_not_replace() {
        let h = Handle::start_null();
        h.send(Command::PlayPaths(vec!["/a.flac".into()])).unwrap();
        wait_playing(&h);
        h.send(Command::Enqueue(vec!["/b.flac".into()])).unwrap();
        thread::sleep(Duration::from_millis(40));
        let s = h.status();
        assert_eq!(s.queue.len(), 2);
        assert_eq!(s.current.as_ref().unwrap().title, "a");
    }

    #[test]
    fn handle_play_pause_stop_drop() {
        let h = Handle::start_null();
        h.send(Command::PlayPaths(vec!["/music/a.flac".into()]))
            .unwrap();
        wait_playing(&h);

        let s = h.status();
        assert!(s.current.is_some());
        assert_eq!(s.current.as_ref().unwrap().title, "a");
        assert_eq!(s.state, PlayState::Playing);

        h.send(Command::Pause).unwrap();
        thread::sleep(Duration::from_millis(40));
        assert_eq!(h.status().state, PlayState::Paused);

        h.send(Command::Stop).unwrap();
        thread::sleep(Duration::from_millis(40));
        assert_eq!(h.status().state, PlayState::Stopped);

        drop(h);
    }
}
