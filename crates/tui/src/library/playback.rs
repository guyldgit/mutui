#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Repeat {
    Off,
    One,
    All,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Repeat::Off => "off",
            Repeat::All => "all",
            Repeat::One => "one",
        }
    }
}

#[derive(Debug)]
pub struct Playback {
    pub shuffle: bool,
    pub repeat: Repeat,
    pub played: Vec<usize>,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            shuffle: false,
            repeat: Repeat::Off,
            played: Vec::new(),
        }
    }
}
