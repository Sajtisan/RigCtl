use std::{
    collections::{HashMap, VecDeque},
    env,
    error::Error,
    fmt, io,
    path::{Path, PathBuf},
};

use rigctl_ipc::{Event, Request, RequestId, Response};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

const MAX_MESSAGE_SIZE: usize = 1024 * 1024;
const SOCKET_FILE_NAME: &str = "rigctld.sock";

#[derive(Debug)]
pub enum IpcClientError {
    Io(io::Error),
    Json(serde_json::Error),
    DaemonUnavailable(io::Error),
    MessageTooLarge,
    UnexpectedEof,
    UnexpectedMessageType(String),
}

impl fmt::Display for IpcClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(f, "IPC I/O error: {error}")
            }
            Self::Json(error) => {
                write!(f, "invalid IPC JSON: {error}")
            }
            Self::DaemonUnavailable(error) => {
                write!(f, "RigCtl daemon unavailable: {error}")
            }
            Self::MessageTooLarge => {
                write!(f, "IPC message exceeds the 1 MiB limit")
            }
            Self::UnexpectedEof => {
                write!(f, "daemon disconnected unexpectedly")
            }
            Self::UnexpectedMessageType(kind) => {
                write!(f, "unexpected IPC message type: {kind}")
            }
        }
    }
}

impl Error for IpcClientError {}

impl From<io::Error> for IpcClientError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for IpcClientError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub struct IpcClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    pending_responses: HashMap<RequestId, Response>,
    pending_events: VecDeque<Event>,
}

impl IpcClient {
    pub async fn connect_default() -> Result<Self, IpcClientError> {
        let socket_path = default_socket_path()?;

        Self::connect(socket_path).await
    }

    pub async fn connect(socket_path: impl AsRef<Path>) -> Result<Self, IpcClientError> {
        let stream = match UnixStream::connect(socket_path).await {
            Ok(stream) => stream,

            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                return Err(IpcClientError::DaemonUnavailable(error));
            }

            Err(error) => {
                return Err(IpcClientError::Io(error));
            }
        };

        let (read_half, write_half) = stream.into_split();

        Ok(Self {
            reader: BufReader::new(read_half),
            writer: write_half,
            pending_responses: HashMap::new(),
            pending_events: VecDeque::new(),
        })
    }

    pub async fn send_request(&mut self, request: &Request) -> Result<(), IpcClientError> {
        let message = serde_json::to_vec(request)?;

        if message.len() > MAX_MESSAGE_SIZE {
            return Err(IpcClientError::MessageTooLarge);
        }

        self.writer.write_all(&message).await?;
        self.writer.write_all(b"\n").await?;

        Ok(())
    }

    pub async fn receive_response(
        &mut self,
        request_id: RequestId,
    ) -> Result<Response, IpcClientError> {
        if let Some(response) = self.pending_responses.remove(&request_id) {
            return Ok(response);
        }

        loop {
            let frame = read_frame(&mut self.reader)
                .await?
                .ok_or(IpcClientError::UnexpectedEof)?;

            let value: Value = serde_json::from_slice(&frame)?;

            let message_type = value
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| IpcClientError::UnexpectedMessageType("<missing>".to_string()))?;

            match message_type {
                "response" => {
                    let response: Response = serde_json::from_value(value)?;

                    let response_id = response_id(&response);

                    if response_id == request_id {
                        return Ok(response);
                    }

                    self.pending_responses.insert(response_id, response);
                }

                "event" => {
                    let event: Event = serde_json::from_value(value)?;

                    self.pending_events.push_back(event);
                }

                other => {
                    return Err(IpcClientError::UnexpectedMessageType(other.to_string()));
                }
            }
        }
    }

    pub async fn request(&mut self, request: &Request) -> Result<Response, IpcClientError> {
        self.send_request(request).await?;
        self.receive_response(request.id).await
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        self.pending_events.pop_front()
    }
}

fn response_id(response: &Response) -> RequestId {
    match response {
        Response::Success(response) => response.id,
        Response::Error(response) => response.id,
    }
}

fn default_socket_path() -> Result<PathBuf, IpcClientError> {
    let xdg_runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        IpcClientError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR is not set",
        ))
    })?;

    Ok(PathBuf::from(xdg_runtime_dir)
        .join("rigctl")
        .join(SOCKET_FILE_NAME))
}

async fn read_frame<R>(reader: &mut BufReader<R>) -> Result<Option<Vec<u8>>, IpcClientError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut frame = Vec::new();

    loop {
        let available = reader.fill_buf().await?;

        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }

            return Err(IpcClientError::UnexpectedEof);
        }

        if let Some(newline_index) = available.iter().position(|byte| *byte == b'\n') {
            if frame.len() + newline_index > MAX_MESSAGE_SIZE {
                return Err(IpcClientError::MessageTooLarge);
            }

            frame.extend_from_slice(&available[..newline_index]);

            reader.consume(newline_index + 1);

            return Ok(Some(frame));
        }

        if frame.len() + available.len() > MAX_MESSAGE_SIZE {
            return Err(IpcClientError::MessageTooLarge);
        }

        let consumed = available.len();

        frame.extend_from_slice(available);
        reader.consume(consumed);
    }
}
