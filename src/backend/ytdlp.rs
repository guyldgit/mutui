use crate::picker::{SearchHit, SourceKind};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Debug)]
pub enum YtError {
    MissingBinary,
    Command(String),
    Parse(String),
    NoFile,
}

impl std::fmt::Display for YtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YtError::MissingBinary => write!(f, "yt-dlp not found in PATH"),
            YtError::Command(s) => write!(f, "yt-dlp: {s}"),
            YtError::Parse(s) => write!(f, "yt-dlp json: {s}"),
            YtError::NoFile => write!(f, "yt-dlp produced no file"),
        }
    }
}

impl std::error::Error for YtError {}

#[derive(Deserialize)]
struct Flat {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    webpage_url: Option<String>,
}

pub fn search(kind: SourceKind, query: &str, n: u32) -> Result<Vec<SearchHit>, YtError> {
    let prefix = match kind {
        SourceKind::Youtube => format!("ytsearch{n}:{query}"),
        SourceKind::Soundcloud => format!("scsearch{n}:{query}"),
        SourceKind::Files => return Ok(vec![]),
    };

    let out = run(&[
        "--flat-playlist",
        "-j",
        "--skip-download",
        "--no-warnings",
        "--no-playlist",
        &prefix,
    ])?;

    let mut hits = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let row: Flat = 
            serde_json::from_str(line).map_err(|e| YtError::Parse(e.to_string()))?;
        let url = row
            .webpage_url
            .or(row.url)
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        hits.push(SearchHit {
            title: row.title.unwrap_or_else(|| url.clone()),
            url,
        });
    }
    Ok(hits)
}

pub fn download(url: &str) -> Result<PathBuf, YtError> {
    let dir = cache_dir();
    fs::create_dir_all(&dir).map_err(|e| YtError::Command(e.to_string()))?;

    let template = dir.join("%(id)s.%(ext)s");
    let out = run(&[
        "-f",
        "bestaudio[ext=m4a]/bestaudio[ext=mp3]/bestaudio[ext=opus]/bestaudio",
        "--no-playlist",
        "--no-warnings",
        "--print",
        "after_move:filepath",
        "-o",
        template.to_str().unwrap(),
        url,
    ])?;

    let path = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .last()
        .map(PathBuf::from)
        .ok_or(YtError::NoFile)?;

    if !path.exists() {
        return Err(YtError::NoFile);
    }
    Ok(path)
}

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("mutui")
}

fn run(args: &[&str]) -> Result<String, YtError> {
    let output = Command::new("yt-dlp")
        .args(args)
        .output()
        .map_err(|_| YtError::MissingBinary)?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(YtError::Command(err.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[allow(dead_code)]
pub fn _timeout() -> Duration {
    Duration::from_secs(120)
}
