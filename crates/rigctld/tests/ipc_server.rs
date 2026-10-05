use std::os::unix::fs::PermissionsExt;

use rigctl_ipc::{Response, SuccessResponse};
use rigctld::ipc::IpcServer;
use tempfile::tempdir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

#[tokio::test]
async fn binds_socket_with_expected_permissions() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let directory_metadata = std::fs::metadata(&runtime_dir).expect("runtime directory missing");

    let directory_mode = directory_metadata.permissions().mode() & 0o777;

    assert_eq!(directory_mode, 0o700);

    let socket_metadata = std::fs::metadata(server.socket_path()).expect("socket missing");

    let socket_mode = socket_metadata.permissions().mode() & 0o777;

    assert_eq!(socket_mode, 0o600);
}

#[tokio::test]
async fn accepts_unix_socket_connection() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let socket_path = server.socket_path().to_path_buf();

    let accept_task = tokio::spawn(async move {
        server
            .accept()
            .await
            .expect("server failed to accept connection")
    });

    let client = UnixStream::connect(&socket_path)
        .await
        .expect("client failed to connect");

    let server_connection = accept_task.await.expect("accept task panicked");

    drop(client);
    drop(server_connection);
}

#[tokio::test]
async fn receives_request_over_unix_socket() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let socket_path = server.socket_path().to_path_buf();

    let accept_task = tokio::spawn(async move {
        let mut connection = server
            .accept()
            .await
            .expect("server failed to accept connection");

        connection
            .read_request()
            .await
            .expect("server failed to read request")
            .expect("client disconnected before sending request")
    });

    let mut client = UnixStream::connect(&socket_path)
        .await
        .expect("client failed to connect");

    client
        .write_all(
            b"{\"version\":1,\"type\":\"request\",\"id\":42,\"method\":\"mouse.dpi.get\",\"params\":{}}\n",
        )
        .await
        .expect("client failed to send request");

    let request = accept_task.await.expect("server task panicked");

    assert_eq!(request.version, 1);
    assert_eq!(request.id, 42);
    assert_eq!(request.method, "mouse.dpi.get");
    assert!(request.params.is_empty());
}

#[tokio::test]
async fn persistent_connection_receives_multiple_requests() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let socket_path = server.socket_path().to_path_buf();

    let accept_task = tokio::spawn(async move {
        let mut connection = server
            .accept()
            .await
            .expect("server failed to accept connection");

        let first = connection
            .read_request()
            .await
            .expect("failed to read first request")
            .expect("expected first request");

        let second = connection
            .read_request()
            .await
            .expect("failed to read second request")
            .expect("expected second request");

        (first, second)
    });

    let mut client = UnixStream::connect(&socket_path)
        .await
        .expect("client failed to connect");

    client
        .write_all(
            b"{\"version\":1,\"type\":\"request\",\"id\":1,\"method\":\"mouse.dpi.get\",\"params\":{}}\n\
{\"version\":1,\"type\":\"request\",\"id\":2,\"method\":\"mouse.polling_rate.get\",\"params\":{}}\n",
        )
        .await
        .expect("client failed to send requests");

    let (first, second) = accept_task.await.expect("server task panicked");

    assert_eq!(first.id, 1);
    assert_eq!(first.method, "mouse.dpi.get");

    assert_eq!(second.id, 2);
    assert_eq!(second.method, "mouse.polling_rate.get");
}

#[tokio::test]
async fn sends_response_over_unix_socket() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let socket_path = server.socket_path().to_path_buf();

    let server_task = tokio::spawn(async move {
        let mut connection = server.accept().await.expect("failed to accept connection");

        let request = connection
            .read_request()
            .await
            .expect("failed to read request")
            .expect("expected request");

        let response = Response::Success(SuccessResponse::new(request.id, serde_json::Map::new()));

        connection
            .send_response(&response)
            .await
            .expect("failed to send response");
    });

    let mut client = UnixStream::connect(&socket_path)
        .await
        .expect("failed to connect");

    client
        .write_all(
            b"{\"version\":1,\"type\":\"request\",\"id\":42,\"method\":\"system.status\",\"params\":{}}\n",
        )
        .await
        .expect("failed to send request");

    let mut reader = BufReader::new(client);

    let mut line = String::new();

    reader
        .read_line(&mut line)
        .await
        .expect("failed to read response");

    let response: Response = serde_json::from_str(line.trim_end()).expect("invalid response");

    match response {
        Response::Success(response) => {
            assert_eq!(response.id, 42);
        }
        Response::Error(_) => {
            panic!("expected successful response");
        }
    }

    server_task.await.expect("server task panicked");
}

#[tokio::test]
async fn malformed_client_does_not_break_listener() {
    let temp = tempdir().expect("failed to create temporary directory");

    let runtime_dir = temp.path().join("rigctl");

    let server = IpcServer::bind(&runtime_dir).expect("failed to bind IPC server");

    let socket_path = server.socket_path().to_path_buf();

    let server_task = tokio::spawn(async move {
        let mut bad_connection = server.accept().await.expect("failed to accept bad client");

        assert!(bad_connection.read_request().await.is_err());

        drop(bad_connection);

        let mut good_connection = server.accept().await.expect("failed to accept good client");

        good_connection
            .read_request()
            .await
            .expect("valid request failed")
            .expect("expected valid request")
    });

    let mut bad_client = UnixStream::connect(&socket_path)
        .await
        .expect("bad client failed to connect");

    bad_client
        .write_all(b"{not valid json}\n")
        .await
        .expect("failed to send malformed request");

    drop(bad_client);

    let mut good_client = UnixStream::connect(&socket_path)
        .await
        .expect("good client failed to connect");

    good_client
        .write_all(
            b"{\"version\":1,\"type\":\"request\",\"id\":99,\"method\":\"system.status\",\"params\":{}}\n",
        )
        .await
        .expect("failed to send valid request");

    let request = server_task.await.expect("server task panicked");

    assert_eq!(request.id, 99);
}
