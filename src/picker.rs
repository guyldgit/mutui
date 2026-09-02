use std::fs;
use std::path::Path;

const EXTS: &[&str] = &["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SourceKind {
    Youtube,
    Soundcloud,
    Files,
}

impl SourceKind {
    pub fn title(self) -> &'static str {
        match self {
            SourceKind::Youtube => "YouTube",
            SourceKind::Soundcloud => "SoundCloud",
            SourceKind::Files => "Files",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub title: String,
    pub url: String, // page url OR filesystem path
}

pub struct Picker {
    pub source: SourceKind,
    pub prompt: String,
    pub results: Vec<SearchHit>,
    pub selected: usize,
    pub status: String,
    all: Vec<SearchHit>,
}

impl Picker {
    pub fn new(source: SourceKind) -> Self {
        Self {
            source,
            prompt: String::new(),
            results: Vec::new(),
            selected: 0,
            status: String::new(),
            all: Vec::new(),
        }
    }

    pub fn new_files(root: &Path) -> Self {
        let all = walk_audio(root);
        let n = all.len();
        Self {
            source: SourceKind::Files,
            prompt: String::new(),
            results: all.clone(),
            selected: 0,
            status: format!("{n} files"),
            all,
        }
    }

    pub fn type_char(&mut self, c: char) {
        self.prompt.push(c);
        self.refilter();
    }

    pub fn backspace(&mut self) {
        self.prompt.pop();
        self.refilter();
    }

    pub fn move_sel(&mut self, delta: isize) {
        if self.results.is_empty() {
            return;
        }
        let n = self.results.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
    }

    pub fn confirm(&mut self) -> Option<SearchHit> {
        self.results.get(self.selected).cloned()
    }

    pub fn needs_search(&self) -> bool {
        matches!(self.source, SourceKind::Youtube | SourceKind::Soundcloud)
            && self.results.is_empty()
    }

    fn refilter(&mut self) {
        if self.source != SourceKind::Files {
            return;
        }
        let q = self.prompt.to_lowercase();
        self.results = self
            .all
            .iter()
            .filter(|h| {
                q.is_empty()
                    || h.title.to_lowercase().contains(&q)
                    || h.url.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        self.selected = 0;
        self.status = format!("{} files", self.results.len());
    }
}

fn walk_audio(root: &Path) -> Vec<SearchHit> {
    let mut out = Vec::new();
    fn rec(dir: &Path, root: &Path, out: &mut Vec<SearchHit>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                rec(&p, root, out);
            } else if is_audio(&p) {
                let rel = p.strip_prefix(root).unwrap_or(&p);
                out.push(SearchHit {
                    title: rel.display().to_string(),
                    url: p.to_string_lossy().into_owned(),
                });
            }
        }
    }
    rec(root, root, &mut out);
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTS.iter().any(|ok| e.eq_ignore_ascii_case(ok)))
        .unwrap_or(false)
}
