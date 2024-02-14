

use std::{io, result};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    InvalidData(String),
    RecursionLimitExceeded,
}
pub type Result<T> = result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}


#[macro_export]
macro_rules! invalid_data {
    ($($arg:tt)*) => {
        $crate::serialize::Error::InvalidData(format!($($arg)*))
    }
}

pub use invalid_data;