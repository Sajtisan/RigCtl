use std::{error::Error, fmt, io};

use rigctl_ipc::{Request, Response};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

use super::framing::{FrameError, MAX_MESSAGE_SIZE, read_frame};

#[derive(Debug)]
pub enum RequestReadError {
    Frame(String),
    InvalidJson(serde_json::Error),
}

impl fmt::Display for RequestReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(error) => {
                write!(f, "IPC framing error: {error}")
            }
            Self::InvalidJson(error) => {
                write!(f, "invalid IPC JSON: {error}")
            }
        }
    }
}

impl Error for RequestReadError {}

impl From<FrameError> for RequestReadError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error.to_string())
    }
}

impl From<serde_json::Error> for RequestReadError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidJson(error)
    }
}

#[derive(Debug)]
pub enum ResponseWriteError {
    Io(io::Error),
    Serialize(serde_json::Error),
    MessageTooLarge,
}

impl fmt::Display for ResponseWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(f, "IPC write error: {error}")
            }
            Self::Serialize(error) => {
                write!(f, "failed to serialize IPC response: {error}")
            }
            Self::MessageTooLarge => {
                write!(f, "IPC response exceeds the 1 MiB limit")
            }
        }
    }
}

impl Error for ResponseWriteError {}

impl From<io::Error> for ResponseWriteError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ResponseWriteError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

pub(crate) async fn read_request<R>(
    reader: &mut BufReader<R>,
) -> Result<Option<Request>, RequestReadError>
where
    R: AsyncRead + Unpin,
{
    let Some(frame) = read_frame(reader).await? else {
        return Ok(None);
    };

    let request = serde_json::from_slice::<Request>(&frame)?;

    Ok(Some(request))
}

pub(crate) async fn write_response<W>(
    writer: &mut W,
    response: &Response,
) -> Result<(), ResponseWriteError>
where
    W: AsyncWrite + Unpin,
{
    let message = serde_json::to_vec(response)?;

    if message.len() > MAX_MESSAGE_SIZE {
        return Err(ResponseWriteError::MessageTooLarge);
    }

    writer.write_all(&message).await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parses_valid_request() {
        let input =
            b"{\"version\":1,\"type\":\"request\",\"id\":42,\"method\":\"mouse.dpi.set\",\"params\":{\"dpi\":800}}\n";

        let mut reader = BufReader::new(&input[..]);

        let request = read_request(&mut reader)
            .await
            .expect("request read failed")
            .expect("expected a request");

        assert_eq!(request.version, 1);
        assert_eq!(request.id, 42);
        assert_eq!(request.method, "mouse.dpi.set");
        assert_eq!(request.params.get("dpi"), Some(&serde_json::json!(800)),);
    }

    #[tokio::test]
    async fn rejects_malformed_json() {
        let input = b"{\"version\":1,\"type\":\"request\",\n";

        let mut reader = BufReader::new(&input[..]);

        let error = read_request(&mut reader)
            .await
            .expect_err("malformed JSON should fail");

        assert!(matches!(error, RequestReadError::InvalidJson(_)));
    }

    #[tokio::test]
    async fn rejects_wrong_message_type() {
        let input =
            b"{\"version\":1,\"type\":\"event\",\"id\":42,\"method\":\"mouse.dpi.get\",\"params\":{}}\n";

        let mut reader = BufReader::new(&input[..]);

        let error = read_request(&mut reader)
            .await
            .expect_err("event must not deserialize as request");

        assert!(matches!(error, RequestReadError::InvalidJson(_)));
    }

    #[tokio::test]
    async fn clean_disconnect_returns_none() {
        let input = b"";

        let mut reader = BufReader::new(&input[..]);

        let request = read_request(&mut reader)
            .await
            .expect("clean disconnect should not fail");

        assert!(request.is_none());
    }
}
