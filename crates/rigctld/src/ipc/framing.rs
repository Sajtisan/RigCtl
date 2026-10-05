use std::{error::Error, fmt, io};

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

pub(crate) const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Debug)]
pub(crate) enum FrameError {
    Io(io::Error),
    MessageTooLarge,
    UnexpectedEof,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Io(err) => write!(f, "I/O error: {}", err),
            FrameError::MessageTooLarge => write!(f, "Message exceeds maximum size (1 MiB)"),
            FrameError::UnexpectedEof => {
                write!(f, "Connection closed before the IPC message was complete")
            }
        }
    }
}

impl Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(err: io::Error) -> Self {
        FrameError::Io(err)
    }
}

pub(crate) async fn read_frame<R>(reader: &mut BufReader<R>) -> Result<Option<Vec<u8>>, FrameError>
where
    R: AsyncRead + Unpin,
{
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            return Err(FrameError::UnexpectedEof);
        }

        if let Some(newline_index) = available.iter().position(|byte| *byte == b'\n') {
            if frame.len() + newline_index > MAX_MESSAGE_SIZE {
                return Err(FrameError::MessageTooLarge);
            }

            frame.extend_from_slice(&available[..newline_index]);
            reader.consume(newline_index + 1);
            return Ok(Some(frame));
        }

        if frame.len() + available.len() > MAX_MESSAGE_SIZE {
            return Err(FrameError::MessageTooLarge);
        }

        let consumed = available.len();
        frame.extend_from_slice(available);
        reader.consume(consumed);
    }
}

// "Unit tests of private implementation details"

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_complete_frame() {
        let input = b"{\"id\":42}\n";

        let mut reader = BufReader::new(&input[..]);

        let frame = read_frame(&mut reader)
            .await
            .expect("frame read failed")
            .expect("expected a frame");

        assert_eq!(frame, b"{\"id\":42}");
    }

    #[tokio::test]
    async fn reconstructs_frame_across_multiple_reads() {
        let input = b"{\"id\":42,\"method\":\"mouse.dpi.get\"}\n";

        let mut reader = BufReader::with_capacity(4, &input[..]);

        let frame = read_frame(&mut reader)
            .await
            .expect("frame read failed")
            .expect("expected a frame");

        assert_eq!(frame, b"{\"id\":42,\"method\":\"mouse.dpi.get\"}");
    }

    #[tokio::test]
    async fn preserves_multiple_frames() {
        let input = b"{\"id\":1}\n{\"id\":2}\n";

        let mut reader = BufReader::new(&input[..]);

        let first = read_frame(&mut reader)
            .await
            .expect("first frame failed")
            .expect("expected first frame");

        let second = read_frame(&mut reader)
            .await
            .expect("second frame failed")
            .expect("expected second frame");

        assert_eq!(first, b"{\"id\":1}");
        assert_eq!(second, b"{\"id\":2}");
    }

    #[tokio::test]
    async fn rejects_oversized_frame() {
        let mut input = vec![b'a'; MAX_MESSAGE_SIZE + 1];
        input.push(b'\n');

        let mut reader = BufReader::new(&input[..]);

        let error = read_frame(&mut reader)
            .await
            .expect_err("oversized frame should fail");

        assert!(matches!(error, FrameError::MessageTooLarge));
    }

    #[tokio::test]
    async fn rejects_partial_frame_at_eof() {
        let input = b"{\"id\":42";

        let mut reader = BufReader::new(&input[..]);

        let error = read_frame(&mut reader)
            .await
            .expect_err("partial frame should fail");

        assert!(matches!(error, FrameError::UnexpectedEof));
    }

    #[tokio::test]
    async fn clean_eof_returns_none() {
        let input = b"";

        let mut reader = BufReader::new(&input[..]);

        let frame = read_frame(&mut reader)
            .await
            .expect("EOF should not be an error");

        assert!(frame.is_none());
    }
}
