use std::{io, string::FromUtf8Error};

#[derive(Debug)]
pub enum OmfError {
    Io(io::Error),
    Utf(FromUtf8Error),
    Value(&'static str),
}

impl From<io::Error> for OmfError {
    fn from(value: io::Error) -> Self {
        OmfError::Io(value)
    }
}

impl From<FromUtf8Error> for OmfError {
    fn from(value: FromUtf8Error) -> Self {
        OmfError::Utf(value)
    }
}