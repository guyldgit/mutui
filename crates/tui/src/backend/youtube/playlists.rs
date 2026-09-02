use super::oauth;
use crate::picker::SearchHit;
use serde::Deserialize;

#[derive(Deserialize)]
struct PlaylistList {
    items: Option<Vec<PlaylistItem>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct PlaylistItem {
    id: Option<String>,
    snippet: Option<Snippet>,
}

#[derive(Deserialize)]
struct Snippet {
    title: Option<String>,
    #[serde(rename = "resourceId")]
    resource_id: Option<ResourceId>,
}

#[derive(Deserialize)]
struct ResourceId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

pub fn list_mine() -> Result<Vec<SearchHit>, String> {
    let token = oauth::access_token()?;
    let http = reqwest::blocking::Client::new();
    let mut out = Vec::new();
    let mut page: Option<String> = None;

    loop {
        let mut url = String::from(
            "https://www.googleapis.com/youtube/v3/playlists?part=snippet&mine=true&maxResults=50",
        );
        if let Some(p) = &page {
            url.push_str("&pageToken=");
            url.push_str(p);
        }
        let body: PlaylistList = http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .map_err(|e| e.to_string())?;

        for it in body.items.unwrap_or_default() {
            let id = it.id.unwrap_or_default();
            let title = it
                .snippet
                .and_then(|s| s.title)
                .unwrap_or_else(|| id.clone());
            out.push(SearchHit {
                title,
                url: format!("playlist:{id}"),
            });
        }

        page = body.next_page_token;
        if page.is_none() {
            break;
        }
    }
    Ok(out)
}

pub fn list_items(playlist_id: &str) -> Result<Vec<SearchHit>, String> {
    let token = oauth::access_token()?;
    let http = reqwest::blocking::Client::new();
    let mut out = Vec::new();
    let mut page: Option<String> = None;

    loop {
        let mut url = format!(
            "https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&maxResults=50&playlistId={playlist_id}"
        );
        if let Some(p) = &page {
            url.push_str("&pageToken=");
            url.push_str(p);
        }
        let body: PlaylistList = http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .map_err(|e| e.to_string())?;

        for it in body.items.unwrap_or_default() {
            let title = it
                .snippet
                .as_ref()
                .and_then(|s| s.title.clone())
                .unwrap_or_else(|| "untitled".into());
            let Some(vid) = it
                .snippet
                .as_ref()
                .and_then(|s| s.resource_id.as_ref())
                .and_then(|r| r.video_id.clone())
            else {
                continue;
            };
            out.push(SearchHit {
                title,
                url: format!("https://www.youtube.com/watch?v={vid}"),
            });
        }

        page = body.next_page_token;
        if page.is_none() {
            break;
        }
    }
    Ok(out)
}
