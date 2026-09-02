use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

const SCOPE: &str = "https://www.googleapis.com/auth/youtube.readonly";
const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";

#[derive(Deserialize)]
struct ClientFile {
    installed: ClientInner,
}

#[derive(Deserialize)]
struct ClientInner {
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

pub fn client_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("musicli/google_client.json")
}

pub fn token_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("musicli/youtube_token.json")
}

fn load_client() -> Result<ClientInner, String> {
    let raw = fs::read_to_string(client_path())
        .map_err(|_| format!("missing {}", client_path().display()))?;
    let file: ClientFile =
        serde_json::from_str(&raw).map_err(|e| format!("google_client.json: {e}"))?;
    Ok(file.installed)
}

pub fn load_token() -> Option<TokenSet> {
    let raw = fs::read_to_string(token_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_token(t: &TokenSet) -> Result<(), String> {
    if let Some(dir) = token_path().parent() {
        fs::create_dir_all(dir).ok();
    }
    fs::write(
        token_path(),
        serde_json::to_string_pretty(t).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn access_token() -> Result<String, String> {
    let client = load_client()?;
    let Some(tok) = load_token() else {
        return Err("not logged in — use <leader>ay".into());
    };
    refresh_if_needed(&client, tok)
}

fn refresh_if_needed(client: &ClientInner, tok: TokenSet) -> Result<String, String> {
    if let Some(refresh) = &tok.refresh_token {
        let http = reqwest::blocking::Client::new();
        let resp: TokenResponse = http
            .post(TOKEN)
            .form(&[
                ("client_id", client.client_id.as_str()),
                ("client_secret", client.client_secret.as_str()),
                ("refresh_token", refresh.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .map_err(|e| e.to_string())?;

        let new = TokenSet {
            access_token: resp.access_token.clone(),
            refresh_token: resp.refresh_token.or(tok.refresh_token),
            expires_in: resp.expires_in,
        };
        save_token(&new)?;
        return Ok(new.access_token);
    }
    Ok(tok.access_token)
}

pub fn login() -> Result<(), String> {
    let client = load_client()?;

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let verifier = pkce_verifier();
    let challenge = pkce_challenge(&verifier);
    let state = pkce_verifier();

    let url = format!(
        "{AUTH}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        urlencoding(&client.client_id),
        urlencoding(&redirect),
        urlencoding(SCOPE),
        urlencoding(&state),
        urlencoding(&challenge),
    );

    open_browser(&url);

    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let path = first.split_whitespace().nth(1).unwrap_or("");
    let qs = path.split('?').nth(1).unwrap_or("");
    let code = query_param(qs, "code").ok_or("no code in redirect")?;
    let got_state = query_param(qs, "state").unwrap_or_default();
    if got_state != state {
        return Err("oauth state mismatch".into());
    }

    let body = b"musicli: you can close this tab";
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);

    let http = reqwest::blocking::Client::new();
    let resp: TokenResponse = http
        .post(TOKEN)
        .form(&[
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| format!("token exchange: {e}"))?
        .json()
        .map_err(|e| e.to_string())?;

    save_token(&TokenSet {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_in: resp.expires_in,
    })
}

fn pkce_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn urlencoding(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                o.push(b as char)
            }
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

fn query_param(qs: &str, key: &str) -> Option<String> {
    for part in qs.split('&') {
        let mut it = part.splitn(2, '=');
        if it.next()? == key {
            let v = it.next().unwrap_or("");
            return Some(v.replace('+', " "));
        }
    }
    None
}

fn open_browser(url: &str) {
    if webbrowser::open(url).is_err() {
        eprintln!("open this URL to log in:\n{url}");
    }
}
