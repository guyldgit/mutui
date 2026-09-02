mod audio;

mod command;
mod engine;
mod error;
mod handle;
mod player;
mod queue;
mod track;
mod worker;
mod library;
mod playlist;

pub use command::{Command, Event, PlayState, Status, Repeat};
pub use error::{Error, Result};
pub use handle::Handle;
pub use player::Player;
pub use queue::Queue;
pub use track::{SourceId, Track, TrackId};
pub use library::{is_audio_path, scan_dir};
pub use playlist::{load as load_playlist, save_queue, save_paths};

pub fn core_alive() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};
    use std::path::PathBuf;

    #[test]
    fn each_variant_displays() {
        let cases: Vec<Error> = vec![
            IoError::new(ErrorKind::NotFound, "missing").into(),
            Error::Device("no output".into()),
            Error::Decode("bad frame".into()),
            Error::InvalidPath {
                path: PathBuf::from("/nope"),
            },
            Error::QueueEmpty,
            Error::ChannelClosed,
        ];

        for err in &cases {
            let s = err.to_string();
            assert!(!s.is_empty(), "{err:?} displayed empty");
        }
    }

    #[test]
    fn io_from_works() {
        let io = IoError::new(ErrorKind::PermissionDenied, "denied");
        let err: Error = io.into();
        match err {
            Error::Io(_) => {}
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn result_alias_rejects() {
        fn boom() -> Result<()> {
            Err(Error::QueueEmpty)
        }
        assert!(boom().is_err());
    }
}
