use rigctl_ipc::{Request, Response};
use tokio::{
    io::BufReader,
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use super::codec::{RequestReadError, ResponseWriteError, read_request, write_response};

pub struct IpcConnection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl IpcConnection {
    pub(crate) fn new(stream: UnixStream) -> Self {
        let (read_half, write_half) = stream.into_split();

        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        }
    }

    pub async fn read_request(&mut self) -> Result<Option<Request>, RequestReadError> {
        read_request(&mut self.reader).await
    }

    pub async fn send_response(&mut self, response: &Response) -> Result<(), ResponseWriteError> {
        write_response(&mut self.writer, response).await
    }
}
