use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub struct Highlight {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
}

impl Highlight {
    pub fn fg(color: Color) -> Self {
        Self {
            fg: Some(color),
            bg: None,
            modifier: Modifier::empty(),
        }
    }

    pub fn style(self) -> Style {
        let mut s = Style::default();

        if let Some(fg) = self.fg {
            s = s.fg(fg);
        }

        if let Some(bg) = self.bg {
            s = s.bg(bg);
        }

        s = s.add_modifier(self.modifier);
        s
    }

}

#[derive(Clone, Debug)]
pub struct Theme {
    pub normal: Highlight,
    pub title: Highlight,
    pub border: Highlight,
    pub muted: Highlight,
    pub playing: Highlight,
    pub paused: Highlight,
    pub stopped: Highlight,
    pub volume: Highlight,
    pub progress: Highlight,
    pub selected: Highlight,
    pub list: Highlight,
}

impl Theme {
    pub fn default_dark() -> Self {
        Self {
            normal: Highlight::fg(Color::Rgb(205, 214, 244)),
            title: Highlight {
                fg: Some(Color::Rgb(137, 180, 250)),
                bg: None,
                modifier: Modifier::BOLD,
            },
            border: Highlight::fg(Color::Rgb(69, 71, 90)),
            muted: Highlight::fg(Color::Rgb(108, 112, 134)),
            playing: Highlight::fg(Color::Rgb(166, 227, 161)),
            paused: Highlight::fg(Color::Rgb(249, 226, 175)),
            stopped: Highlight::fg(Color::Rgb(108, 112, 134)),
            volume: Highlight::fg(Color::Rgb(166, 227, 161)),
            progress: Highlight::fg(Color::Rgb(249, 226, 175)),
            selected: Highlight {
                fg: Some(Color::Rgb(30, 30, 46)),
                bg: Some(Color::Rgb(137, 180, 250)),
                modifier: Modifier::BOLD,
            },
            list: Highlight::fg(Color::Rgb(205, 214, 244)),
        }
    }

    pub fn get(&self, name: &str) -> Highlight {
        match name {
            "Normal" => self.normal,
            "Title" => self.title,
            "Border" => self.border,
            "Muted" => self.muted,
            "Playing" => self.playing,
            "Paused" => self.paused,
            "Stopped" => self.stopped,
            "Volume" => self.volume,
            "Progress" => self.progress,
            "Selected" => self.selected,
            "List" => self.list,
            _ => self.normal,
        }
    }

    pub fn set(&mut self, name: &str, hl: Highlight) {
        match name {
            "Normal" => self.normal = hl,
            "Title" => self.title = hl,
            "Border" => self.border = hl,
            "Muted" => self.muted = hl,
            "Playing" => self.playing = hl,
            "Paused" => self.paused = hl,
            "Stopped" => self.stopped = hl,
            "Volume" => self.volume = hl,
            "Progress" => self.progress = hl,
            "Selected" => self.selected = hl,
            "List" => self.list = hl,
            _ => {}
        }
    }
}

pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    Some(match s.to_ascii_lowercase().as_str() {
        "reset" | "none" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "white" => Color::White,
        _ => return None,
    })
}
