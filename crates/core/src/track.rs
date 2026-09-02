use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::error::{Error, Result};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackId(u64);

impl TrackId {
    fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceId {
    Local,
}

impl SourceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceId::Local => "local",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub path: PathBuf,
    pub source: SourceId,
    pub duration: Option<Duration>,
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Track {}

impl Track {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(Error::InvalidPath {
                path: path.to_path_buf(),
            });
        }

        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        Ok(Self {
            id: TrackId::next(),
            title,
            path: path.to_path_buf(),
            source: SourceId::Local,
            duration: None,
        })
    }

    pub fn display_title(&self) -> &str {
        &self.title
    }
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_path_uses_stem_as_title() {
        let t = Track::from_path("/music/ramranch.flac").unwrap();
        assert_eq!(t.title, "ramranch");
        assert_eq!(t.path, PathBuf::from("/music/ramranch.flac"));
        assert_eq!(t.source, SourceId::Local);
        assert!(t.duration.is_none());
        assert_eq!(t.to_string(), "ramranch");
    }

    #[test]
    fn empty_path_is_invalid() {
        let err = Track::from_path("").unwrap_err();
        match err {
            Error::InvalidPath { .. } => {}
            other => panic!("expected InvalidPath, got {other:?}"),
        }
    }

    #[test]
    fn clone_preserves_id() {
        let a = Track::from_path("/a.mp3").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.id, b.id);
        assert_eq!(a.path, b.path);
    }

    #[test]
    fn equality_is_by_id_not_path() {
        let a = Track::from_path("/same.flac").unwrap();
        let b = Track::from_path("/same.flac").unwrap();
        assert_ne!(a.id, b.id);
        assert_ne!(a, b);
    }
}
