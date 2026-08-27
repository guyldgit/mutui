mod player;

use player::{Player, PlayerError};
use ratatui{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        layout::{Constraint, Layout},
        style::{Color, Style, Stylize},
        text::{Line, Span},
        widgets::{Block, Borders, Gauge, Paragraph},
        Frame, Terminal,
};
