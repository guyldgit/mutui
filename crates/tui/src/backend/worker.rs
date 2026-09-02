use super::ytdlp;
use crate::picker::{SearchHit, SourceKind};
use crate::library::TrackId;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::path::PathBuf;

pub enum Job {
    Search { kind: SourceKind, query: String },
    Fetch {
        id: TrackId,
        hit: SearchHit,
        source: SourceKind,
    },
    YoutubeLogin,
    YoutubePlaylists,
    YoutubePlaylistItems { id: String },
}

pub enum WorkerEvent {
    SearchDone(Vec<SearchHit>),
    SearchErr(String),
    FetchDone { id: TrackId, path: PathBuf },
    FetchErr { id: TrackId, err: String },
    LoginDone,
    LoginErr(String),
    PlaylistsDone(Vec<SearchHit>),
    PlaylistItemsDone(Vec<SearchHit>),
    CatalogErr(String),
}

pub fn spawn() -> (Sender<Job>, Receiver<WorkerEvent>) {
    let (job_tx, job_rx) = mpsc::channel::<Job>();
    let (ev_tx, ev_rx) = mpsc::channel::<WorkerEvent>();

    thread::spawn(move || {
        while let Ok(job) = job_rx.recv() {
            let ev = match job {
                Job::Search { kind, query } => match ytdlp::search(kind, &query, 10) {
                    Ok(hits) => WorkerEvent::SearchDone(hits),
                    Err(e) => WorkerEvent::SearchErr(e.to_string()),
                },
                Job::Fetch { id, hit, source: _ } => match ytdlp::download(&hit.url) {
                    Ok(path) => WorkerEvent::FetchDone { id, path },
                    Err(e) => WorkerEvent::FetchErr { id, err: e.to_string() },
                },
                Job::YoutubeLogin => match crate::backend::youtube::oauth::login() {
                    Ok(()) => WorkerEvent::LoginDone,
                    Err(e) => WorkerEvent::LoginErr(e),
                },
                Job::YoutubePlaylists => match crate::backend::youtube::playlists::list_mine() {
                    Ok(h) => WorkerEvent::PlaylistsDone(h),
                    Err(e) => WorkerEvent::CatalogErr(e),
                },
                Job::YoutubePlaylistItems { id } => {
                    match crate::backend::youtube::playlists::list_items(&id) {
                        Ok(h) => WorkerEvent::PlaylistItemsDone(h),
                        Err(e) => WorkerEvent::CatalogErr(e),
                    }
                },
            };
            if ev_tx.send(ev).is_err() {
                break;
            }
        }
    });

    (job_tx, ev_rx)
}
