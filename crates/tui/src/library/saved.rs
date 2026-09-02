use crate::library::{Playable, Track};
use crate::picker::SourceKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct SavedPlaylist {
    pub name: String,
    pub tracks: Vec<SavedTrack>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedTrack {
    pub title: String,
    pub source: String,
    pub location: String,
}

impl SavedTrack {
    pub fn from_track(t: &Track) -> Self {
        let (source, location) = match t.source {
            SourceKind::Youtube => (
                "youtube",
                t.origin.clone().unwrap_or_default(),
            ),
            SourceKind::Soundcloud => (
                "soundcloud",
                t.origin.clone().unwrap_or_else(|| match &t.playable {
                    Playable::Url(u) => u.clone(),
                    Playable::File(p) => p.display().to_string(),
                }),
            ),
            SourceKind::Files => (
                "files",
                t.path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
        };
        Self {
            title: t.title.clone(),
            source: source.into(),
            location,
        }
    }

    pub fn into_track(self) -> Track {
        match self.source.as_str() {
            "youtube" => Track::youtube(self.title, self.location),
            "soundcloud" => Track::soundcloud(self.title, self.location),
            _ => Track::local(PathBuf::from(self.location)),
        }
    }
}

pub fn dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mutui/playlists")
}

pub fn path_for(name: &str) -> PathBuf {
    dir().join(format!("{name}.json"))
}

pub fn save(name: &str, tracks: &[Track]) -> Result<(), String> {
    fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let pl = SavedPlaylist {
        name: name.into(),
        tracks: tracks.iter().map(SavedTrack::from_track).collect(),
    };
    let raw = serde_json::to_string_pretty(&pl).map_err(|e| e.to_string())?;
    fs::write(path_for(name), raw).map_err(|e| e.to_string())
}

pub fn load(name: &str) -> Result<Vec<Track>, String> {
    let raw = fs::read_to_string(path_for(name)).map_err(|e| e.to_string())?;
    let pl: SavedPlaylist = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(pl.tracks.into_iter().map(SavedTrack::into_track).collect())
}

pub fn list() -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir()) else { return vec![] };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| e.path().file_stem()?.to_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    names
}
