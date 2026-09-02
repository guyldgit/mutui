use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("audio device: {0}")]
    Device(String),

    #[error("decode failed: {0}")]
    Decode(String),

    #[error("invalid path: {path}")]
    InvalidPath { path: PathBuf },

    #[error("queue is empty")]
    QueueEmpty,

    #[error("player channel closed")]
    ChannelClosed,

    #[error("queue index {index} out of range (len {len})")]
    InvalidIndex { index: usize, len: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
