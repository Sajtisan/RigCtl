use std::{error::Error, fmt};

use rigctl_ipc::Request;
use tokio::io::{AsyncRead, BufReader};

use super::framing::{FrameError, read_frame};

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
