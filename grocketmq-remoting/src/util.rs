use thiserror::Error;

pub fn vec_to_u32(data: &[u8]) -> u32 {
    let mut result = 0;
    result |= (data[0] as u32) << 24;
    result |= (data[1] as u32) << 16;
    result |= (data[2] as u32) << 8;
    result |= data[3] as u32;
    result
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad command data")]
    DecodeCommandError,
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("stream not ready")]
    StreamNotReady,
    #[error("invalid address {0}")]
    InvalidAddress(String),
}
