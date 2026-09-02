use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub const MAX_BYTES: u64 = 500 * 1024 * 1024;

pub fn dir() -> PathBuf {
    std::env::temp_dir().join("mutui")
}

pub fn lookup(key: &str) -> Option<PathBuf> {
    let dir = dir();
    let rd = fs::read_dir(&dir).ok()?;
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.file_stem().and_then(|s| s.to_str()) == Some(key) {
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub fn bind_cached(track: &mut crate::library::Track) {
    if matches!(track.playable, crate::library::Playable::File(_)) {
        return;
    }
    if let Some(key) = track.cache_key() {
        if let Some(path) = lookup(&key) {
            track.set_file(path);
        }
    }
}

pub fn enforce_cap(keep: &[PathBuf]) {
    let dir = dir();
    let Ok(rd) = fs::read_dir(&dir) else { return };

    let mut files: Vec<(PathBuf, u64, SystemTime)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((p, meta.len(), meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)))
        })
        .collect();

    files.sort_by_key(|(_, _, t)| *t);

    let mut total: u64 = files.iter().map(|(_, n, _)| *n).sum();
    for (p, n, _) in files {
        if total <= MAX_BYTES {
            break;
        }
        if keep.iter().any(|k| k == &p) {
            continue;
        }
        if fs::remove_file(&p).is_ok() {
            total = total.saturating_sub(n);
        }
    }
}

pub fn wipe() {
    let _ = fs::remove_dir_all(dir());
}
