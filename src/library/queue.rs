use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Queue {
    pub items: Vec<PathBuf>,
    pub current: Option<usize>,
}

impl Queue {
    pub fn push(&mut self, path: PathBuf) {
        self.items.push(path);
    }

    pub fn remove_selected(&mut self, selected: usize) {
        if selected >= self.items.len() {
            return;
        }
        self.items.remove(selected);
        if let Some(cur) = self.current {
            if selected < cur {
                self.current = Some(cur - 1);
            } else if selected == cur {
                self.current = None;
            }
        }
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current.and_then(|i| self.items.get(i)).map(|p| p.as_path())
    }

    pub fn play_index(&mut self, i: usize) -> Option<PathBuf> {
        if i < self.items.len() {
            self.current = Some(i);
            Some(self.items[i].clone())
        } else {
            None
        }
    }

    pub fn advance(&mut self) -> Option<PathBuf> {
        let next = self.current.map(|i| i + 1).unwrap_or(0);
        self.play_index(next)
    }
}
