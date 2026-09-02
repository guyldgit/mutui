use crate::player::Player;
use crate::library::{Library, Queue};
use crate::picker::Picker;
use std::sync::mpsc::{Receiver, Sender};
use crate::backend::{Job, WorkerEvent};

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Library,
    Queue,
}

pub enum InputMode {
    Normal,
    Leader { acc: String },
    Picker,
}

pub struct App {
    pub player: Player,
    pub library: Library,
    pub queue: Queue,
    pub focus: Focus,
    pub queue_sel: usize,
    pub mode: InputMode,
    pub picker: Option<Picker>,
    pub jobs: Sender<Job>,
    pub events: Receiver<WorkerEvent>,
}
