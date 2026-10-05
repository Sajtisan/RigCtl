use std::{
    env, fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use tokio::net::UnixListener;

use super::connection::IpcConnection;

const RUNTIME_DIRECTORY_NAME: &str = "rigctl";
const SOCKET_FILE_NAME: &str = "rigctld.sock";

pub struct IpcServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn bind_defaults() -> io::Result<Self> {
        let runtime_dir = default_runtime_dir()?;
        Self::bind(runtime_dir)
    }

    pub fn bind(runtime_dir: impl AsRef<Path>) -> io::Result<Self> {
        let runtime_dir = runtime_dir.as_ref();
        fs::create_dir_all(runtime_dir)?;
        fs::set_permissions(runtime_dir, fs::Permissions::from_mode(0o700))?;

        let socket_path = runtime_dir.join(SOCKET_FILE_NAME);
        let listener = UnixListener::bind(&socket_path)?;

        if let Err(error) = fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600)) {
            drop(listener);
            let _ = fs::remove_file(&socket_path);
            return Err(error);
        }

        Ok(Self {
            listener,
            socket_path,
        })
    }

    pub async fn accept(&self) -> io::Result<IpcConnection> {
        let (stream, _) = self.listener.accept().await?;

        Ok(IpcConnection::new(stream))
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

fn default_runtime_dir() -> io::Result<PathBuf> {
    let xdg_runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR environment variable is not set",
        )
    })?;
    Ok(PathBuf::from(xdg_runtime_dir).join(RUNTIME_DIRECTORY_NAME))
}
