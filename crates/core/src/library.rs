use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::track::Track;

const AUDIO_EXT: &[&str] = &["flac", "mp3", "ogg", "opus", "wav", "m4a", "aac"];

pub fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXT.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

pub fn scan_dir(root: impl AsRef<Path>) -> Result<Vec<Track>> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(Error::InvalidPath {
            path: root.to_path_buf(),
        });
    }

    let mut paths = Vec::new();
    collect(root, &mut paths)?;
    paths.sort_by(|a, b| {
        let na = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let nb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        natural_cmp(na, nb).then_with(|| a.cmp(b))
    });

    paths.into_iter().map(Track::from_path).collect()
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(&path, out)?;
        } else if is_audio_path(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let (mut i, mut j) = (0usize, 0usize);

    while i < ac.len() && j < bc.len() {
        if ac[i].is_ascii_digit() && bc[j].is_ascii_digit() {
            let take = |s: &[char], k: &mut usize| {
                let mut n = 0u64;
                while *k < s.len() && s[*k].is_ascii_digit() {
                    n = n * 10 + s[*k].to_digit(10).unwrap() as u64;
                    *k += 1;
                }
                n
            };
            let ord = take(&ac, &mut i).cmp(&take(&bc, &mut j));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        } else {
            let ord = ac[i]
                .to_lowercase()
                .cmp(bc[j].to_lowercase());
            i += 1;
            j += 1;
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
    }
    ac.len().cmp(&bc.len())
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
    fn scan_sorts_naturally_and_skips_non_audio() {
        let d = scratch("scan");
        fs::write(d.join("track10.flac"), b"").unwrap();
        fs::write(d.join("track2.flac"), b"").unwrap();
        fs::write(d.join("readme.txt"), b"").unwrap();
        fs::create_dir(d.join(".hidden")).unwrap();
        fs::write(d.join(".hidden").join("x.mp3"), b"").unwrap();

        let tracks = scan_dir(&d).unwrap();
        let names: Vec<_> = tracks
            .iter()
            .map(|t| t.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["track2.flac", "track10.flac"]);
    }
}
