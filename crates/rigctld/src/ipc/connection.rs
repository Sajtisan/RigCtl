use rigctl_ipc::Request;
use tokio::{
    io::BufReader,
    net::{UnixStream, unix::OwnedReadHalf},
};

use super::codec::{RequestReadError, read_request};

pub struct IpcConnection {
    reader: BufReader<OwnedReadHalf>,
}

impl IpcConnection {
    pub(crate) fn new(stream: UnixStream) -> Self {
        let (read_half, _write_half) = stream.into_split();

        Self {
            reader: BufReader::new(read_half),
        }
    }

    pub async fn read_request(&mut self) -> Result<Option<Request>, RequestReadError> {
        read_request(&mut self.reader).await
    }
}
