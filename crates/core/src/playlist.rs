use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::queue::Queue;
use crate::track::Track;

pub fn save_queue(path: impl AsRef<Path>, queue: &Queue) -> Result<()> {
    save_paths(path, queue.tracks().iter().map(|t| t.path.as_path()))
}

pub fn save_paths<'a>(
    path: impl AsRef<Path>,
    paths: impl IntoIterator<Item = &'a Path>,
) -> Result<()> {
    let path = path.as_ref();
    let mut f = fs::File::create(path)?;
    writeln!(f, "# mutui playlist")?;
    for p in paths {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir()?.join(p)
        };
        writeln!(f, "{}", abs.display())?;
    }
    Ok(())
}

pub fn load(path: impl AsRef<Path>) -> Result<Vec<Track>> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or(Path::new("."));
    let f = fs::File::open(path)?;
    let mut tracks = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let p = PathBuf::from(line);
        let p = if p.is_absolute() {
            p
        } else {
            parent.join(p)
        };
        tracks.push(Track::from_path(p)?);
    }
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mutui-08-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn playlist_roundtrip() {
        let d = scratch("pl");
        let a = d.join("a.mp3");
        let b = d.join("b.mp3");
        fs::write(&a, b"").unwrap();
        fs::write(&b, b"").unwrap();
        let list = d.join("q.mutui");

        save_paths(&list, [a.as_path(), b.as_path()]).unwrap();
        let loaded = load(&list).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].title, "a");
        assert_eq!(loaded[1].title, "b");
    }
}
