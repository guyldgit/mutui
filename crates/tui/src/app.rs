use crate::library::{Library, Queue, Playback, TrackId};
use crate::picker::{Picker, SearchHit};
use std::{path::PathBuf, sync::mpsc::{Receiver, Sender}};
use crate::backend::{Job, WorkerEvent};
use std::collections::HashSet;
use mutui_core::{Handle, Status};

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Library,
    Playlists,
    Queue,
}

pub enum InputMode {
    Normal,
    Leader { acc: String },
    Picker,
}

pub struct App {
    pub handle: Handle,
    pub status: Status,
    pub library: Library,
    pub queue: Queue,
    pub focus: Focus,
    pub queue_sel: usize,
    pub mode: InputMode,
    pub picker: Option<Picker>,
    pub jobs: Sender<Job>,
    pub events: Receiver<WorkerEvent>,
    pub playback: Playback,
    pub playlists: Vec<SearchHit>,
    pub playlist_sel: usize,
    pub pending_playlist: bool,
    pub prefetch: usize,
    pub in_flight: HashSet<TrackId>,
    pub awaiting_play: Option<TrackId>,
    pub enqueued_paths: HashSet<PathBuf>,
}
