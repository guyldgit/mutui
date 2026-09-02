use crate::error::{Error, Result};
use crate::track::{Track, TrackId};

#[derive(Debug, Clone, Default)]
pub struct Queue {
    tracks: Vec<Track>,
    current: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    pub fn current(&self) -> Result<&Track> {
        let idx = self.current.ok_or(Error::QueueEmpty)?;
        self.tracks.get(idx).ok_or(Error::QueueEmpty)
    }

    pub fn current_mut(&mut self) -> Result<&mut Track> {
        let idx = self.current.ok_or(Error::QueueEmpty)?;
        self.tracks.get_mut(idx).ok_or(Error::QueueEmpty)
    }

    pub fn current_id(&self) -> Result<TrackId> {
        Ok(self.current()?.id)
    }

    pub fn push(&mut self, track: Track) {
        self.tracks.push(track);
        if self.current.is_none() {
            self.current = Some(0);
        }
    }

    pub fn insert(&mut self, index: usize, track: Track) -> Result<()> {
        if index > self.tracks.len() {
            return Err(Error::InvalidIndex {
                index,
                len: self.tracks.len(),
            });
        }
        self.tracks.insert(index, track);
        match self.current {
            None => self.current = Some(index.min(self.tracks.len() - 1)),
            Some(i) if index <= i => self.current = Some(i + 1),
            Some(_) => {}
        }
        Ok(())
    }

    pub fn set_current(&mut self, index: usize) -> Result<&Track> {
        if index >= self.tracks.len() {
            return Err(Error::InvalidIndex {
                index,
                len: self.tracks.len(),
            });
        }
        self.current = Some(index);
        Ok(&self.tracks[index])
    }

    pub fn next(&mut self) -> Result<&Track> {
        if self.tracks.is_empty() {
            self.current = None;
            return Err(Error::QueueEmpty);
        }
        let i = self.current.unwrap_or(0);
        let n = (i + 1) % self.tracks.len();
        self.current = Some(n);
        Ok(&self.tracks[n])
    }

    pub fn prev(&mut self) -> Result<&Track> {
        if self.tracks.is_empty() {
            self.current = None;
            return Err(Error::QueueEmpty);
        }
        let i = self.current.unwrap_or(0);
        let n = if i == 0 {
            self.tracks.len() - 1
        } else {
            i - 1
        };
        self.current = Some(n);
        Ok(&self.tracks[n])
    }
    
    pub fn remove(&mut self, index: usize) -> Result<Track> {
        if index >= self.tracks.len() {
            return Err(Error::InvalidIndex {
                index,
                len: self.tracks.len(),
            });
        }
        let removed = self.tracks.remove(index);

        self.current = match (self.current, self.tracks.len()) {
            (_, 0) => None,
            (None, _) => None,
            (Some(c), len) if index == c => Some(index.min(len - 1)),
            (Some(c), _) if index < c => Some(c - 1),
            (Some(c), _) => Some(c),
        };

        Ok(removed)
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(name: &str) -> Track {
        Track::from_path(format!("/music/{name}.flac")).unwrap()
    }

    fn titles(q: &Queue) -> Vec<&str> {
        q.tracks().iter().map(|x| x.title.as_str()).collect()
    }

    fn cur(q: &Queue) -> &str {
        q.current().unwrap().title.as_str()
    }

    #[test]
    fn push_sets_current_on_first() {
        let mut q = Queue::new();
        assert!(q.current().is_err());
        q.push(t("a"));
        q.push(t("b"));
        assert_eq!(cur(&q), "a");
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn next_wraps() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.push(t("b"));
        q.push(t("c"));
        assert_eq!(q.next().unwrap().title, "b");
        assert_eq!(q.next().unwrap().title, "c");
        assert_eq!(q.next().unwrap().title, "a");
    }

    #[test]
    fn prev_wraps_at_zero() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.push(t("b"));
        q.push(t("c"));
        assert_eq!(q.prev().unwrap().title, "c");
        assert_eq!(q.prev().unwrap().title, "b");
    }

    #[test]
    fn empty_next_prev() {
        let mut q = Queue::new();
        assert!(matches!(q.next().unwrap_err(), Error::QueueEmpty));
        assert!(matches!(q.prev().unwrap_err(), Error::QueueEmpty));
    }

    #[test]
    fn remove_current_prefers_following() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.push(t("b"));
        q.push(t("c"));
        q.set_current(1).unwrap();
        let gone = q.remove(1).unwrap();
        assert_eq!(gone.title, "b");
        assert_eq!(cur(&q), "c");
        assert_eq!(titles(&q), ["a", "c"]);
    }

    #[test]
    fn remove_last_when_current() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.push(t("b"));
        q.push(t("c"));
        q.set_current(2).unwrap();
        q.remove(2).unwrap();
        assert_eq!(cur(&q), "b");
    }

    #[test]
    fn remove_before_current_shifts_index() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.push(t("b"));
        q.push(t("c"));
        q.set_current(2).unwrap();
        q.remove(0).unwrap();
        assert_eq!(cur(&q), "c");
        assert_eq!(q.current_index(), Some(1));
    }

    #[test]
    fn remove_only_track() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.remove(0).unwrap();
        assert!(q.is_empty());
        assert!(q.current().is_err());
    }

    #[test]
    fn clear() {
        let mut q = Queue::new();
        q.push(t("a"));
        q.clear();
        assert!(q.is_empty());
        assert!(matches!(q.next().unwrap_err(), Error::QueueEmpty));
    }

    #[test]
    fn set_current_oob() {
        let mut q = Queue::new();
        q.push(t("a"));
        let err = q.set_current(3).unwrap_err();
        assert!(matches!(err, Error::InvalidIndex { index: 3, len: 1 }));
    }
}
