//! To allow standard GC errors to be detected and handled no matter the implementation, this module
//! provides a simple error type partially modeled after `std::io::Error`.

use std::error;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    /// This error indicates that there is no longer enough memory to fill an allocation request.
    OutOfMemory,
    /// Unlike the out of memory error, this error indicates that a given request could never be
    /// filled under the current configuration and may indicate a programming issue. An example of
    /// this would be an allocation exceeding the size of the heap.
    ///
    /// Some GCs may choose to return OutOfMemory instead in some cases due to performance reasons.
    AllocationTooLarge,
    /// Used to indicate an requested alignment is unsupported. For example, some GCs may round up
    /// to a fixed alignment for all allocations and are unable to support custom allocations larger
    /// than the programmed amount.
    UnsupportedAlignment,
    /// This error indicates that an invalid state has been reached by the GC. This may be the
    /// result of unsafe code or be produced by a GC which requires strict usage requirements.
    IllegalState,
    /// This error occurs when attempting to access an object that has already been garbage
    /// collected. It should not be assumed that this error will be returned as not many garbage
    /// collectors attempt to detect when this occurs.
    UseAfterFree,
    /// Any error which is not covered by another error kind.
    Other,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::OutOfMemory => write!(f, "Not enough memory remaining to complete request"),
            ErrorKind::AllocationTooLarge => {
                write!(f, "Allocation exceeds the maximum allowed size for this GC")
            }
            ErrorKind::UnsupportedAlignment => {
                write!(f, "The requested allocation alignment is not supported")
            }
            ErrorKind::IllegalState => write!(
                f,
                "Attempted to enter an invalid state to complete this request"
            ),
            ErrorKind::UseAfterFree => {
                write!(f, "Attempted to access an object which has been freed")
            }
            ErrorKind::Other => write!(
                f,
                "An unknown error occurred while attempting to complete the request"
            ),
        }
    }
}

pub struct Error {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            kind,
            error: error.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind, format!("{}", kind))
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {:?}", &self.kind, &self.error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Other => Display::fmt(&self.error, f),
            x => write!(f, "{:?}: {}", x, &self.error),
        }
    }
}
