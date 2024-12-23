
use hidapi::HidError;

/// A possible error value when opening the sensor or taking a reading.
#[derive(Debug)]
pub enum Error {

    /// A hardware access error.
    Hid(Box<HidError>),

    /// The sensor returned an invalid message or a single read timeout
    /// expired.
    InvalidMessage,

    /// A checksum error.
    Checksum,

    /// The sensor did not report all values before the timeout expired.
    ///
    /// Note that this can only occur when calling
    /// [`Sensor::read`][crate::Sensor::read].
    /// [`Sensor::read_one`][crate::Sensor::read_one] returns
    /// [`Error::InvalidMessage`] on timeout.
    Timeout,

    /// The configured timeout was too large.
    InvalidTimeout,

}

impl From<HidError> for Error {
    fn from(err: HidError) -> Self {
        Error::Hid(Box::new(err))
    }
}

