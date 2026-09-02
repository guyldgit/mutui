use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use mutui_core::{scan_dir, Command, Handle, PlayState};

#[derive(Parser)]
#[command(
    name = "mutui",
    about = "Local-first player. Core + CLI; TUI is a client.",
    after_help = "Phase 2: `play` and `tui` own the device in this process.\n\
                  Other verbs need a running session (Phase 6 daemon). \
                  Volume is 0–100."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Play {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    Pause,
    Resume,
    Toggle,
    Stop,
    Next,
    Prev,
    Volume { level: u8 },
    Queue {
        #[command(subcommand)]
        action: QueueCmd,
    },
    Tui,
}

#[derive(Subcommand)]
enum QueueCmd {
    Add { paths: Vec<PathBuf> },
    List,
    Clear,
}

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            exit_code(&e)
        }
    };
    std::process::exit(code);
}

fn exit_code(err: &mutui_core::Error) -> i32 {
    match err {
        mutui_core::Error::InvalidPath { .. }
        | mutui_core::Error::QueueEmpty
        | mutui_core::Error::InvalidIndex { .. } => 1,
        _ => 2,
    }
}

fn run() -> mutui_core::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Play { paths } => cmd_play(paths),
        Cmd::Tui => cmd_tui(),
        Cmd::Queue {
            action: QueueCmd::List,
        } => {
            println!("queue is empty (no daemon yet)");
            Ok(())
        }
        Cmd::Volume { .. }
        | Cmd::Pause
        | Cmd::Resume
        | Cmd::Toggle
        | Cmd::Stop
        | Cmd::Next
        | Cmd::Prev
        | Cmd::Queue { .. } => Err(mutui_core::Error::Device(
            "nothing playing; use `mutui play` or `mutui tui` (daemon is Phase 6)".into(),
        )),
    }
}

fn expand(paths: Vec<PathBuf>) -> mutui_core::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            for t in scan_dir(&p)? {
                out.push(t.path);
            }
        } else if p.is_file() {
            out.push(p);
        } else {
            return Err(mutui_core::Error::InvalidPath { path: p });
        }
    }
    if out.is_empty() {
        return Err(mutui_core::Error::QueueEmpty);
    }
    Ok(out)
}

fn cmd_play(paths: Vec<PathBuf>) -> mutui_core::Result<()> {
    let paths = expand(paths)?;
    let handle = Handle::start()?;
    handle.send(Command::PlayPaths(paths))?;

    let mut seen_playing = false;
    loop {
        thread::sleep(Duration::from_millis(50));
        let s = handle.status();
        match s.state {
            PlayState::Playing | PlayState::Paused => seen_playing = true,
            PlayState::Stopped if seen_playing => break,
            PlayState::Stopped => {
            }
        }
    }
    Ok(())
}

fn cmd_tui() -> mutui_core::Result<()> {
    let handle = Handle::start()?;
    mutui_tui::run(handle)
}
