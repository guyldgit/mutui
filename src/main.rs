mod player;
mod ui;
mod config;
mod library;
mod app;
mod picker;
mod input;
mod backend;

use input::ChordResult;
use picker::{Picker, SourceKind};
use app::{App, Focus, InputMode};
use backend::{Job, WorkerEvent, spawn};
use player::Player;
use config::load_config;
use library::{Library, Queue, Track};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind, KeyEvent, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::{
    env,
    error::Error,
    io::{self, stdout},
    path::PathBuf,
    time::Duration,
    collections::HashMap,
};

fn main() -> Result<(), Box<dyn Error>> {
    let cfg = load_config();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let (jobs, events) = spawn();

    let mut app = App {
        player: Player::new()?,
        library: Library::default(),
        queue: Queue::default(),
        focus: Focus::Library,
        queue_sel: 0,
        mode: InputMode::Normal,
        picker: None,
        jobs,
        events,
    };
    app.player.set_volume(cfg.volume);

    if let Some(path) = cli_path() {
        app.library = Library::open(&path)?;
        if path.is_file() {
            app.queue.push(Track::local(path.to_path_buf()));
            play_current(&mut app)?;
        }
    }

    let result = run(&mut terminal, &mut app, &cfg);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn play_current(app: &mut App) -> Result<(), Box<dyn Error>> {
    if let Some(path) = app.queue.current_path().map(|p| p.to_path_buf()) {
        app.player.play_file(&path)?;
    } else if let Some(path) = app.queue.advance() {
        app.player.play_file(&path)?;
    }
    Ok(())
}

fn play_next(app: &mut App) {
    if let Some(path) = app.queue.advance() {
        let _ = app.player.play_file(&path);
    }
}

fn cli_path() -> Option<PathBuf> {
    env::args().nth(1).map(PathBuf::from)
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    cfg: &config::Config,
) -> Result<(), Box<dyn Error>> {
    loop {
        if app.queue.current.is_some() && app.player.is_empty() && !app.player.is_paused() {
            play_next(app);
        }

        while let Ok(ev) = app.events.try_recv() {
            match ev {
                WorkerEvent::SearchDone(hits) => {
                    if let Some(p) = app.picker.as_mut() {
                        p.results = hits;
                        p.selected = 0;
                        p.status = format!("{} results", p.results.len());
                    }
                }
                WorkerEvent::SearchErr(e) | WorkerEvent::DownloadErr(e) => {
                    if let Some(p) = app.picker.as_mut() {
                        p.status = e;
                    }
                }
                WorkerEvent::DownloadDone(track) => {
                    app.queue.push(track);
                    let idle = app.queue.current.is_none() && app.player.is_empty();
                    if idle {
                        let i = app.queue.items.len() - 1;
                        if let Some(path) = app.queue.play_index(i) {
                            let _ = app.player.play_file(&path);
                        }
                    }
                    app.picker = None;
                    app.mode = InputMode::Normal;
                }
                WorkerEvent::LoginDone => {
                    if let Some(p) = app.picker.as_mut() {
                        p.status = "logged in — <leader>py for playlists".into();
                    }
                }
                WorkerEvent::LoginErr(e) | WorkerEvent::CatalogErr(e) => {
                    if let Some(p) = app.picker.as_mut() {
                        p.status = e;
                    }
                }
                WorkerEvent::PlaylistsDone(hits) | WorkerEvent::PlaylistItemsDone(hits) => {
                    if let Some(p) = app.picker.as_mut() {
                        p.results = hits;
                        p.selected = 0;
                        p.status = format!("{} items", p.results.len());
                    }
                }
            }
        }

        terminal.draw(|f| ui::draw(f, app, &cfg.theme, &cfg.keys))?;

        if !event::poll(Duration::from_millis(2))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let Some(token) = key_token(key) else { continue };

        match &mut app.mode {
            InputMode::Picker => {
                handle_picker(app, &token, &cfg.picker_keys);
                continue;
            }
            InputMode::Leader { acc } => {
                match input::feed_leader(acc, &token, &cfg.leader_keys) {
                    ChordResult::Continue => {}
                    ChordResult::Cancel => app.mode = InputMode::Normal,
                    ChordResult::Action(action) => {
                        app.mode = InputMode::Normal;
                        dispatch(app, &action);
                    }
                }
                continue;
            }
            InputMode::Normal => {
                if token == cfg.leader {
                    app.mode = InputMode::Leader { acc: String::new() };
                    continue;
                }
                if let Some(action) = cfg.keys.get(&token).cloned() {
                    if action == "quit" {
                        break;
                    }
                    dispatch(app, &action);
                }
            }
        }
    }
    Ok(())
}


fn key_token(key: KeyEvent) -> Option<String> {
    if key.code == KeyCode::Backspace
        || matches!(key.code, KeyCode::Char('\u{8}' | '\u{7f}'))
    {
        return Some("backspace".into());
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    Some(match key.code {
        KeyCode::Char(c) if ctrl => format!("c-{}", c.to_ascii_lowercase()),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Esc => "esc".into(),
        _ => return None,
    })
}

fn move_sel(app: &mut App, delta: isize) {
    match app.focus {
        Focus::Library => app.library.move_sel(delta),
        Focus::Queue => {
            if app.queue.items.is_empty() {
                return;
            }
            let n = app.queue.items.len() as isize;
            app.queue_sel = (app.queue_sel as isize + delta).rem_euclid(n) as usize;
        }
    }
}
fn handle_picker(app: &mut App, token: &str, picker_keys: &HashMap<String, String>) {
    let action = picker_keys.get(token).map(|s| s.as_str());

    match action {
        Some("picker_close") => {
            app.picker = None;
            app.mode = InputMode::Normal;
        }
        Some("picker_confirm") => {
            let Some(picker) = app.picker.as_mut() else { return };

            if picker.needs_search() {
                if picker.prompt.trim().is_empty() {
                    picker.status = "type a query".into();
                    return;
                }
                picker.status = "searching...".into();
                let _ = app.jobs.send(Job::Search {
                    kind: picker.source,
                    query: picker.prompt.trim().to_string(),
                });
                return;
            }

            let Some(hit) = picker.confirm() else { return };

            if let Some(id) = hit.url.strip_prefix("playlist:") {
                picker.status = "loading tracks...".into();
                let _ = app.jobs.send(Job::YoutubePlaylistItems { id: id.to_string() });
                return;
            }

            match picker.source {
                SourceKind::Files => {
                    let track = Track::local(PathBuf::from(&hit.url));
                    app.queue.push(track);
                    let idle = app.queue.current.is_none() && app.player.is_empty();
                    if idle {
                        let i = app.queue.items.len() - 1;
                        if let Some(p) = app.queue.play_index(i) {
                            let _ = app.player.play_file(&p);
                        }
                    }
                    app.picker = None;
                    app.mode = InputMode::Normal;
                }
                SourceKind::Youtube | SourceKind::Soundcloud => {
                    picker.status = "downloading...".into();
                    let _ = app.jobs.send(Job::Download {
                        source: picker.source,
                        hit,
                    });
                }
            }
        }
        Some("picker_backspace") => {
            if let Some(p) = app.picker.as_mut() {
                p.backspace();
            }
        }
        Some("picker_down") => {
            if let Some(p) = app.picker.as_mut() {
                p.move_sel(1);
            }
        }
        Some("picker_up") => {
            if let Some(p) = app.picker.as_mut() {
                p.move_sel(-1);
            }
        }
        _ => {
            if token.len() == 1 {
                if let Some(c) = token.chars().next() {
                    if !c.is_control() {
                        if let Some(p) = app.picker.as_mut() {
                            p.type_char(c);
                        }
                    }
                }
            }
        }
    }
}

fn dispatch(app: &mut App, action: &str) {
    match action {
        "quit" => {}
        "toggle_pause" => app.player.toggle_pause(),
        "volume_up" => app.player.set_volume(app.player.volume() + 0.05),
        "volume_down" => app.player.set_volume(app.player.volume() - 0.05),
        "search_youtube" => {
            app.picker = Some(Picker::new(SourceKind::Youtube));
            app.mode = InputMode::Picker;
        }
        "search_soundcloud" => {
            app.picker = Some(Picker::new(SourceKind::Soundcloud));
            app.mode = InputMode::Picker;
        }
        "search_files" => {
            let root = app
                .library
                .dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("."));
            app.picker = Some(Picker::new_files(&root));
            app.mode = InputMode::Picker;
        }
        "restart" => {
            if let Some(p) = app.player.current_path().map(|p| p.to_path_buf()) {
                let _ = app.player.play_file(&p);
            }
        }
        "sel_down" => move_sel(app, 1),
        "sel_up" => move_sel(app, -1),
        "focus_next" => {
            app.focus = match app.focus {
                Focus::Library => Focus::Queue,
                Focus::Queue => Focus::Library,
            };
        }
        "queue_add" => {
            if let Some(p) = app.library.selected_path() {
                app.queue.push(Track::local(p.to_path_buf()));
            }
        }
        "queue_remove" => {
            if app.focus == Focus::Queue {
                app.queue.remove_selected(app.queue_sel);
                if app.queue_sel > 0 && app.queue_sel >= app.queue.items.len() {
                    app.queue_sel = app.queue.items.len().saturating_sub(1);
                }
            }
        }
        "play_selected" => {
            match app.focus {
                Focus::Library => {
                    if let Some(p) = app.library.selected_path() {
                        app.queue.push(Track::local(p.to_path_buf()));
                        let i = app.queue.items.len() - 1;
                        if let Some(path) = app.queue.play_index(i) {
                            let _ = app.player.play_file(&path);
                        }
                    }
                }
                Focus::Queue => {
                    if let Some(path) = app.queue.play_index(app.queue_sel) {
                        let _ = app.player.play_file(&path);
                    }
                }
            }
        }
        "next" => play_next(app),
        "youtube_login" => {
            app.picker = Some(Picker::new(SourceKind::Youtube));
            if let Some(p) = app.picker.as_mut() {
                p.status = "opening browser...".into();
            }
            app.mode = InputMode::Picker;
            let _ = app.jobs.send(Job::YoutubeLogin);
        }
        "youtube_playlists" => {
            app.picker = Some(Picker::new(SourceKind::Youtube));
            if let Some(p) = app.picker.as_mut() {
                p.status = "loading playlists...".into();
            }
            app.mode = InputMode::Picker;
            let _ = app.jobs.send(Job::YoutubePlaylists);
        }
        _ => {}
    }
}
