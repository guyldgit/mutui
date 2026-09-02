use crate::input::{ChordResult, feed_leader};
use crate::picker::{Picker, SourceKind, SearchHit};
use crate::app::{App, Focus, InputMode};
use crate::backend::{Job, WorkerEvent, spawn};
use crate::config::load_config;
use crate::library::{Library, Queue, Track, Playback, Repeat, Playable};
use crate::ui;
use crate::config::{Config};
use mutui_core::{Command, Handle, PlayState, Status};
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
    collections::{HashMap, HashSet},
};
use rand::Rng;

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

fn silence_audio_stderr() {
    unsafe {
        std::env::set_var("JACK_NO_START_SERVER", "1");
    }

    let log = dirs::state_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("mutui/audio.log");
    if let Some(dir) = log.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&log) {
        unsafe {
            libc::dup2(file.as_raw_fd(), 2);
        }
    }
}

pub fn run(handle: Handle) -> mutui_core::Result<()> {
    silence_audio_stderr();
    let cfg = load_config();

    enable_raw_mode().map_err(|e| mutui_core::Error::Device(e.to_string()))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| mutui_core::Error::Device(e.to_string()))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))
        .map_err(|e| mutui_core::Error::Device(e.to_string()))?;

    let (jobs, events) = spawn();

    let mut app = App {
        handle,
        status: Status::empty(),
        library: Library::default(),
        queue: Queue::default(),
        focus: Focus::Library,
        queue_sel: 0,
        mode: InputMode::Normal,
        picker: None,
        jobs,
        events,
        playback: Playback::default(),
        playlists: Vec::new(),
        playlist_sel: 0,
        pending_playlist: false,
        prefetch: cfg.prefetch,
        in_flight: HashSet::new(),
        awaiting_play: None,
        enqueued_paths: HashSet::new(),
    };
    if crate::backend::youtube::oauth::load_token().is_some() {
        let _ = app.jobs.send(Job::YoutubePlaylists);
    }

    let _ = app.handle.send(Command::SetVolume(cfg.volume));

    if let Some(path) = cli_path() {
        app.library = Library::open(&path)?;
        if path.is_file() {
            app.queue.push(Track::local(path.to_path_buf()));
            play_current(&mut app)
                .map_err(|e| mutui_core::Error::Device(e.to_string()))?;
        }
    }

    let result = run_loop(&mut terminal, &mut app, &cfg);

    drop(app);
    crate::backend::cache::wipe();

    disable_raw_mode().map_err(|e| mutui_core::Error::Device(e.to_string()))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| mutui_core::Error::Device(e.to_string()))?;
    terminal.show_cursor().map_err(|e| mutui_core::Error::Device(e.to_string()))?;

    result.map_err(|e| mutui_core::Error::Device(e.to_string()))
}

fn start_index(app: &mut App, i: usize) {
    if app.queue.play_index(i).is_none() {
        return;
    }
    crate::backend::cache::bind_cached(&mut app.queue.items[i]);
    match &app.queue.items[i].playable {
        Playable::File(_) => {
            app.awaiting_play = None;
            play_core_from_tui_queue(app, i);
            crate::backend::fetch::sync_fetches(app);
        }
        Playable::Url(_) => {
            app.awaiting_play = Some(app.queue.items[i].id);
            crate::backend::fetch::sync_fetches(app);
        }
    }
}

fn play_core_from_tui_queue(app: &mut App, want: usize) {
    let paths: Vec<PathBuf> = app
        .queue
        .items
        .iter()
        .filter_map(|t| t.path().map(PathBuf::from))
        .collect();
    if paths.is_empty() {
        return;
    }

    let skip = app
        .queue
        .items
        .iter()
        .take(want)
        .filter(|t| t.path().is_some())
        .count();

    app.enqueued_paths.clear();
    app.enqueued_paths.extend(paths.iter().cloned());

    let _ = app.handle.send(Command::PlayPaths(paths));
    if skip > 0 {
        let _ = app.handle.send(Command::PlayIndex(skip));
    }
}

fn play_current(app: &mut App) -> Result<(), Box<dyn Error>> {
    if let Some(i) = app.queue.current {
        start_index(app, i);
    } else if let Some(i) = next_index(app) {
        start_index(app, i);
    }
    Ok(())
}

fn next_index(app: &mut App) -> Option<usize> {
    let n = app.queue.items.len();
    if n == 0 {
        return None;
    }

    if app.playback.repeat == Repeat::One {
        return app.queue.current;
    }

    if app.playback.shuffle {
        if let Some(i) = app.queue.current {
            if !app.playback.played.contains(&i) {
                app.playback.played.push(i);
            }
        }
        let current = app.queue.current;
        let candidates: Vec<usize> = (0..n)
            .filter(|i| Some(*i) != current && !app.playback.played.contains(i))
            .collect();

        if candidates.is_empty() {
            if app.playback.repeat == Repeat::All {
                app.playback.played.clear();
                return Some(rand::thread_rng().gen_range(0..n));
            }
            return None;
        }
        let i = rand::thread_rng().gen_range(0..candidates.len());
        return Some(candidates[i]);
    }

    let next = app.queue.current.map(|i| i + 1).unwrap_or(0);
    if next < n {
        Some(next)
    } else if app.playback.repeat == Repeat::All {
        Some(0)
    } else {
        None
    }
}

fn cli_path() -> Option<PathBuf> {
    env::args().nth(1).map(PathBuf::from)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    cfg: &Config,
) -> Result<(), Box<dyn Error>> {
    loop {
        app.status = app.handle.status();
        if let Some(cur) = app.status.current.as_ref() {
            if let Some(i) = app
                .queue
                .items
                .iter()
                .position(|t| t.path() == Some(cur.path.as_path()))
            {
                app.queue.current = Some(i);
            }
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
                WorkerEvent::SearchErr(e) => {
                    if let Some(p) = app.picker.as_mut() {
                        p.status = e;
                    }
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
                WorkerEvent::PlaylistsDone(hits) => {
                    app.playlists = hits;
                    app.playlist_sel = 0;
                    if let Some(p) = app.picker.as_mut() {
                        p.results = app.playlists.clone();
                        p.selected = 0;
                        p.status = format!("{} playlists", app.playlists.len());
                    }
                }
                WorkerEvent::PlaylistItemsDone(hits) => {
                    if app.pending_playlist {
                        app.pending_playlist = false;
                        app.queue.items = hits
                            .into_iter()
                            .map(|h| Track::youtube(h.title, h.url))
                            .collect();
                        app.queue.current = None;
                        app.queue_sel = 0;
                        app.playback.played.clear();
                        if !app.queue.items.is_empty() {
                            start_index(app, 0);
                        }
                    } else if let Some(p) = app.picker.as_mut() {
                        p.results = hits;
                        p.selected = 0;
                        p.status = format!("{} tracks", p.results.len());
                    }
                }
                WorkerEvent::FetchDone { id, path } => {
                    app.in_flight.remove(&id);
                    if let Some(t) = app.queue.items.iter_mut().find(|t| t.id == id) {
                        t.set_file(path.clone());
                    }

                    if app.awaiting_play == Some(id) {
                        app.awaiting_play = None;
                        let idx = app
                            .queue
                            .items
                            .iter()
                            .position(|t| t.id == id)
                            .unwrap_or(0);
                        play_core_from_tui_queue(app, idx);
                    } else if app.enqueued_paths.insert(path.clone()) {
                        let _ = app.handle.send(Command::Enqueue(vec![path]));
                    }

                    crate::backend::fetch::sync_fetches(app);
                    let keep: Vec<PathBuf> = app
                        .queue
                        .items
                        .iter()
                        .filter_map(|t| t.path().map(|p| p.to_path_buf()))
                        .collect();
                    crate::backend::cache::enforce_cap(&keep);
                }
                WorkerEvent::FetchErr { id, err } => {
                    app.in_flight.remove(&id);
                    if app.awaiting_play == Some(id) {
                        app.awaiting_play = None;
                    }
                    eprintln!("fetch {id:?}: {err}");
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
                match feed_leader(acc, &token, &cfg.leader_keys) {
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
        Focus::Playlists => {
            if app.playlists.is_empty() {
                return;
            }
            let n = app.playlists.len() as isize;
            app.playlist_sel = (app.playlist_sel as isize + delta).rem_euclid(n) as usize;
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

            if let Some(name) = hit.url.strip_prefix("localpl:") {
                match crate::library::saved::load(name) {
                    Ok(tracks) => {
                        app.queue.items = tracks;
                        app.queue.current = None;
                        app.queue_sel = 0;
                        app.playback.played.clear();
                        app.picker = None;
                        app.mode = InputMode::Normal;
                        if !app.queue.items.is_empty() {
                            start_index(app, 0);
                        }
                    }
                    Err(e) => {
                        if let Some(p) = app.picker.as_mut() {
                            p.status = e;
                        }
                    }
                }
                return;
            }

            match picker.source {
                SourceKind::Files => {
                    let path = PathBuf::from(&hit.url);
                    let track = Track::local(path.clone());
                    app.queue.push(track);
                    let idle = app.queue.current.is_none()
                        && app.status.state == PlayState::Stopped;
                    if idle {
                        start_index(app, app.queue.items.len() - 1);
                    } else if app.enqueued_paths.insert(path.clone()) {
                        let _ = app.handle.send(Command::Enqueue(vec![path]));
                    }
                    app.picker = None;
                    app.mode = InputMode::Normal;
                }
                SourceKind::Youtube | SourceKind::Soundcloud => {
                    let track = match picker.source {
                        SourceKind::Youtube => Track::youtube(hit.title, hit.url),
                        SourceKind::Soundcloud => Track::soundcloud(hit.title, hit.url),
                        SourceKind::Files => unreachable!(),
                    };
                    app.queue.push(track);
                    let idle = app.queue.current.is_none() && app.status.state == PlayState::Stopped;
                    if idle {
                        start_index(app, app.queue.items.len() - 1);
                    }
                    app.picker = None;
                    app.mode = InputMode::Normal;
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
        "toggle_pause" => {
            let _ = app.handle.send(Command::TogglePause);
        }
        "volume_up" => {
            let v = (app.status.volume + 0.05).clamp(0.0, 1.0);
            let _ = app.handle.send(Command::SetVolume(v));
        }
        "volume_down" => {
            let v = (app.status.volume - 0.05).clamp(0.0, 1.0);
            let _ = app.handle.send(Command::SetVolume(v));
        }
        "toggle_shuffle" => {
            let _ = app.handle.send(Command::ToggleShuffle);
        }
        "cycle_repeat" => {
            let r = app.status.repeat.cycle();
            let _ = app.handle.send(Command::SetRepeat(r));
        }
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
            if let Some(cur) = app.queue.current {
                start_index(app, cur);
            }
        }
        "sel_down" => move_sel(app, 1),
        "sel_up" => move_sel(app, -1),
        "focus_next" => {
            app.focus = match app.focus {
                Focus::Library if !app.playlists.is_empty() => Focus::Playlists,
                Focus::Library => Focus::Queue,
                Focus::Playlists => Focus::Queue,
                Focus::Queue => Focus::Library,
            };
        }
        "queue_add" => {
            if let Some(p) = app.library.selected_path() {
                let path = p.to_path_buf();
                app.queue.push(Track::local(path.clone()));
                if app.enqueued_paths.insert(path.clone()) {
                    let _ = app.handle.send(Command::Enqueue(vec![path]));
                }
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
        "play_selected" => match app.focus {
            Focus::Library => {
                if let Some(p) = app.library.enter() {
                    app.queue.push(Track::local(p));
                    let i = app.queue.items.len() - 1;
                    start_index(app, i);
                }
            }
            Focus::Queue => start_index(app, app.queue_sel),
            Focus::Playlists => {
                if let Some(pl) = app.playlists.get(app.playlist_sel) {
                    if let Some(id) = pl.url.strip_prefix("playlist:") {
                        app.pending_playlist = true;
                        let _ = app.jobs.send(Job::YoutubePlaylistItems {
                            id: id.to_string(),
                        });
                    }
                }
            }
        },
        "next" => {
            let _ = app.handle.send(Command::Next);
        }
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
        "playlist_save" => {
            let name = format!(
                "queue-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            );
            if let Err(e) = crate::library::saved::save(&name, &app.queue.items) {
                eprintln!("{e}");
            }
        }
        "playlist_open" => {
            let names = crate::library::saved::list();
            let mut p = Picker::new(SourceKind::Files);
            p.results = names
                .into_iter()
                .map(|n| SearchHit {
                    title: n.clone(),
                    url: format!("localpl:{n}"),
                })
                .collect();
            p.status = format!("{} playlists", p.results.len());
            app.picker = Some(p);
            app.mode = InputMode::Picker;
        }
        _ => {}
    }
}
