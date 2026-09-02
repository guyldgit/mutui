use crate::picker::SourceKind;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Track {
    pub title: String,
    pub path: PathBuf,
    pub source: SourceKind,
}

impl Track {
    pub fn local(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            title,
            path,
            source: SourceKind::Files,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
