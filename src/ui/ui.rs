use crate::app::{App, Focus};
use crate::config::Theme;
use crate::picker::Picker;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Clear},
    Frame,
};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub fn draw(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    keys: &HashMap<String, String>,
) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(5), // now playing
        Constraint::Length(3), // progress
        Constraint::Min(8),    // library | queue
        Constraint::Length(3), // volume
        Constraint::Length(3), // help
    ])
    .split(frame.area());

    let border = |title: &'static str| {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.border.style())
            .title_style(theme.muted.style())
    };

    frame.render_widget(
        Paragraph::new("musicli")
            .style(theme.title.style())
            .block(border("")),
        chunks[0],
    );

    let path_text = app
        .player
        .current_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(nothing loaded)".into());

    let (state, state_hl) = if app.player.is_empty() {
        ("Stopped", theme.stopped)
    } else if app.player.is_paused() {
        ("Paused", theme.paused)
    } else {
        ("Playing", theme.playing)
    };

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("File:  ", theme.muted.style()),
                Span::styled(path_text, theme.normal.style()),
            ]),
            Line::from(vec![
                Span::styled("State: ", theme.muted.style()),
                Span::styled(state, state_hl.style()),
            ]),
        ])
        .block(border("Now Playing")),
        chunks[1],
    );

    let pos = app.player.position();
    let label = match app.player.duration() {
        Some(total) => format!("{} / {}", fmt_time(pos), fmt_time(total)),
        None if !app.player.is_empty() => format!("{} / --:--", fmt_time(pos)),
        None => "--:-- / --:--".into(),
    };
    let area = chunks[2];
    let inner_w = area.width.saturating_sub(2) as usize;
    let bar = render_bar(app.player.progress(), inner_w.saturating_sub(14));

    frame.render_widget(
        Paragraph::new(format!("{bar} {label}"))
            .style(theme.progress.style())
            .block(border("Progress")),
        area,
    );

    let cols = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(chunks[3]);

    let lib_items: Vec<ListItem> = app
        .library
        .entries
        .iter()
        .map(|e| ListItem::new(e.label()).style(theme.list.style()))
        .collect();

    let mut lib_state = ListState::default().with_selected(
        (!app.library.entries.is_empty()).then_some(app.library.selected),
    );

    frame.render_stateful_widget(
        List::new(lib_items)
            .block(
                Block::default()
                    .title(if app.focus == Focus::Library {
                        "Library *"
                    } else {
                        "Library"
                    })
                    .borders(Borders::ALL)
                    .border_style(theme.border.style())
                    .title_style(theme.muted.style()),
            )
            .highlight_style(theme.selected.style()),
        cols[0],
        &mut lib_state,
    );

    let queue_items: Vec<ListItem> = app
        .queue
        .items
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mark = if app.queue.current == Some(i) { "> " } else { "  " };
            ListItem::new(format!("{mark}{}", p.title)).style(theme.list.style())
        })
        .collect();

    let mut queue_state = ListState::default().with_selected(
        (!app.queue.items.is_empty()).then_some(app.queue_sel),
    );

    frame.render_stateful_widget(
        List::new(queue_items)
            .block(
                Block::default()
                    .title(if app.focus == Focus::Queue {
                        "Queue *"
                    } else {
                        "Queue"
                    })
                    .borders(Borders::ALL)
                    .border_style(theme.border.style())
                    .title_style(theme.muted.style()),
            )
            .highlight_style(theme.selected.style()),
        cols[1],
        &mut queue_state,
    );

    let volume = app.player.volume();
    frame.render_widget(
        Gauge::default()
            .block(border("Volume"))
            .gauge_style(theme.volume.style())
            .ratio(volume as f64)
            .label(format!("{:.0}%", volume * 100.0)),
        chunks[4],
    );

    frame.render_widget(
        Paragraph::new(help_text(keys))
            .style(theme.muted.style())
            .block(border("")),
        chunks[5],
    );

    if let Some(picker) = &app.picker {
        draw_picker(frame, picker, theme);
    }
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

fn help_text(keys: &HashMap<String, String>) -> String {
    const ORDER: &[(&str, &str)] = &[
        ("toggle_pause", "play/pause"),
        ("sel_up", "up"),
        ("sel_down", "down"),
        ("play_selected", "play"),
        ("queue_add", "add"),
        ("queue_remove", "del"),
        ("focus_next", "focus"),
        ("next", "next"),
        ("volume_up", "vol+"),
        ("volume_down", "vol-"),
        ("quit", "quit"),
    ];

    ORDER
        .iter()
        .filter_map(|(action, label)| {
            let bound: Vec<&str> = keys
                .iter()
                .filter(|(_, a)| a.as_str() == *action)
                .map(|(k, _)| pretty_key(k))
                .collect();
            if bound.is_empty() {
                None
            } else {
                Some(format!("{} {}", bound.join("/"), label))
            }
        })
        .collect::<Vec<_>>()
        .join("   ")
}

fn render_bar(ratio: f64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let blocks = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
    let total = (ratio.clamp(0.0, 1.0) * width as f64 * 8.0) as usize;
    let full = total / 8;
    let rem = total % 8;
    let mut s = "█".repeat(full);
    if full < width {
        s.push_str(blocks[rem]);
        s.push_str(&" ".repeat(width.saturating_sub(full + 1)));
    }
    s
}

fn draw_picker(frame: &mut Frame, picker: &Picker, theme: &Theme) {
    let area = centered(frame.area(), 70, 50);
    frame.render_widget(Clear, area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    let prompt = format!("> {}", picker.prompt);
    frame.render_widget(
        Paragraph::new(prompt)
            .style(theme.normal.style())
            .block(
                Block::default()
                    .title(picker.source.title())
                    .borders(Borders::ALL)
                    .border_style(theme.border.style())
                    .title_style(theme.title.style()),
            ),
        chunks[0],
    );

    let items: Vec<ListItem> = picker
        .results
        .iter()
        .map(|h| ListItem::new(h.title.clone()).style(theme.list.style()))
        .collect();

    let mut state = ListState::default()
        .with_selected((!picker.results.is_empty()).then_some(picker.selected));

    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border.style()),
            )
            .highlight_style(theme.selected.style())
            .highlight_symbol("▶ "),
        chunks[1],
        &mut state,
    );

    frame.render_widget(
        Paragraph::new(picker.status.as_str())
            .style(theme.muted.style())
            .block(Block::default().borders(Borders::ALL).border_style(theme.border.style())),
        chunks[2],
    );
}

fn centered(area: Rect, w_pct: u16, h_pct: u16) -> Rect {
    let w = area.width * w_pct / 100;
    let h = area.height * h_pct / 100;
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn fmt_time(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn pretty_key(k: &str) -> &str {
    match k {
        " " => "space",
        "esc" => "esc",
        "enter" => "enter",
        "tab" => "tab",
        "up" => "up",
        "down" => "down",
        other => other,
    }
}
