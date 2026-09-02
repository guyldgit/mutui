use crate::picker::SourceKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(u64);

impl TrackId {
    pub fn new() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub enum Playable {
    File(PathBuf),
    Url(String),
}

#[derive(Clone, Debug)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub playable: Playable,
    pub source: SourceKind,
    pub origin: Option<String>,
}

impl Track {
    pub fn local(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            id: TrackId::new(),
            title,
            playable: Playable::File(path),
            source: SourceKind::Files,
            origin: None
        }
    }

    pub fn youtube(title: String, url: String) -> Self {
        Self {
            id: TrackId::new(),
            title,
            origin: Some(url.clone()),
            playable: Playable::Url(url),
            source: SourceKind::Youtube,
        }
    }

    pub fn soundcloud(title: String, url: String) -> Self {
        Self {
            id: TrackId::new(),
            title,
            origin: Some(url.clone()),
            playable: Playable::Url(url),
            source: SourceKind::Soundcloud,
        }
    }

    pub fn cache_key(&self) -> Option<String> {
        match (&self.source, &self.playable) {
            (_, Playable::File(p)) => p.file_stem().map(|s| s.to_string_lossy().into_owned()),
            (SourceKind::Youtube, Playable::Url(u)) => youtube_id(u),
            (SourceKind::Soundcloud, Playable::Url(u)) => Some(u.rsplit('/').next()?.to_string()),
            _ => None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.playable {
            Playable::File(p) => Some(p),
            Playable::Url(_) => None,
        }
    }

    pub fn set_file(&mut self, path: PathBuf) {
        self.playable = Playable::File(path);
    }
}

fn youtube_id(url: &str) -> Option<String> {
    if let Some(i) = url.find("v=") {
        let rest = &url[i + 2..];
        return Some(rest.split('&').next()?.to_string());
    }
    url.rsplit('/').next().map(|s| s.split('?').next().unwrap_or(s).to_string())
}
