//! Failure values.
//!
//! The component never answers the platform with a failure. The framework turns an
//! `Err` from a method into a bare `false`, and 1C reports that as
//! `Ошибка при вызове метода контекста (СоздатьQRКод)` with nothing behind it —
//! no message, no position, nothing a caller can act on. So every method here
//! answers `Ok`, and a failure travels in the value: an empty result, plus the
//! message that the next call to `ТекстПоследнейОшибки` reads back.
//!
//! This module is the shape of that message. It carries a kind as well as a text
//! because the kinds fail for different reasons and a caller may want to tell them
//! apart — a wrong parameter type is the calling script's mistake, while data that
//! does not fit in a QR code is not.

use std::fmt;

use serde::Serialize;

/// Why the component could not produce a code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// A parameter the script passed is of the wrong type, or empty where empty
    /// is meaningless.
    Argument,
    /// The options JSON is malformed, carries an unknown key, or holds a value
    /// outside the range the component accepts.
    Options,
    /// The data cannot be encoded: it is longer than a QR code can hold.
    Data,
    /// A defect in the component itself — the bundled font, or PNG encoding.
    Internal,
}

/// A failure, with the kind that says what sort of thing went wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QrError {
    /// What sort of failure this is.
    pub kind: ErrorKind,
    /// The message, in English, naming the parameter or option and the value that
    /// arrived. The BSL wrapper embeds it in a Russian sentence, because the caller
    /// reads that; keeping the two apart means the wrapper can word the whole
    /// message while the component reports only the fact.
    pub message: String,
}

impl QrError {
    /// Build a failure.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// A parameter the script passed cannot be used.
    pub fn argument(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Argument, message)
    }

    /// An option cannot be used.
    pub fn options(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Options, message)
    }

    /// The data cannot be encoded.
    pub fn data(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Data, message)
    }

    /// The component failed at something that cannot fail for a correct input.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

impl fmt::Display for QrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for QrError {}
