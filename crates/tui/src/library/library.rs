use std::fs;
use std::path::{Path, PathBuf};

const EXTS: &[&str] = &["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];

#[derive(Debug, Clone)]
pub enum Entry {
    Parent(PathBuf),
    Dir(PathBuf),
    File(PathBuf),
}

impl Entry {
    pub fn path(&self) -> &Path {
        match self {
            Entry::Parent(p) | Entry::Dir(p) | Entry::File(p) => p,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Entry::Parent(_) => "..".into(),
            Entry::Dir(p) => format!("{}/", file_name(p)),
            Entry::File(p) => file_name(p),
        }
    }
}

#[derive(Debug, Default)]
pub struct Library {
    pub dir: Option<PathBuf>,
    pub entries: Vec<Entry>,
    pub selected: usize,
}

impl Library {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        if path.is_file() {
            return Ok(Self {
                dir: path.parent().map(Path::to_path_buf),
                entries: vec![Entry::File(path.to_path_buf())],
                selected: 0,
            });
        }
        Ok(Self {
            dir: Some(path.to_path_buf()),
            entries: read_dir(path),
            selected: 0,
        })
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn selected_path(&self) -> Option<&Path> {
        match self.selected()? {
            Entry::File(p) => Some(p),
            Entry::Parent(_) | Entry::Dir(_) => None,
        }
    }

    pub fn enter(&mut self) -> Option<PathBuf> {
        match self.selected()? {
            Entry::Parent(p) | Entry::Dir(p) => {
                let p = p.clone();
                *self = Self::open(&p).ok()?;
                None
            }
            Entry::File(p) => Some(p.clone()),
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.dir.as_ref().and_then(|d| d.parent()) {
            if let Ok(lib) = Self::open(parent) {
                *self = lib;
            }
        }
    }

    pub fn move_sel(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let n = self.entries.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
    }
}

fn read_dir(path: &Path) -> Vec<Entry> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                dirs.push(Entry::Dir(p));
            } else if is_audio(&p) {
                files.push(Entry::File(p));
            }
        }
    }

    let cmp = |a: &Path, b: &Path| {
    a.file_name()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
    };
    dirs.sort_by(|a, b| cmp(a.path(), b.path()));
    files.sort_by(|a, b| cmp(a.path(), b.path()));

    let mut out = Vec::new();
    if let Some(parent) = path.parent() {
        out.push(Entry::Parent(parent.to_path_buf()));
    }
    out.extend(dirs);
    out.extend(files);
    out
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTS.iter().any(|ok| e.eq_ignore_ascii_case(ok)))
        .unwrap_or(false)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
