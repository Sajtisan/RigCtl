# Protocol and IPC transport tests

[Back to the test guide](./README.md)

These suites cover the shared protocol types and daemon-side transport components.
They do not launch the production daemon or operate physical devices.

## Protocol contracts

**Source:** [protocol_v1.rs](../../crates/rigctl-ipc/tests/protocol_v1.rs)

```bash
cargo test -p rigctl-ipc --test protocol_v1
```

These seven tests construct shared Rust types, inspect their JSON representations,
and deserialize them. A round trip checks that the decoded object equals the
original; explicit JSON comparisons also catch accidental wire-format changes.

| Test | Contract verified |
| --- | --- |
| `protocol_version_is_one` | `PROTOCOL_VERSION` is exactly `1` |
| `request_matches_v1_wire_format` | A request has the expected version, type, ID, method, and object-shaped params; it round-trips |
| `success_response_matches_v1_wire_format` | Success uses `status: "ok"` with `result`; it round-trips |
| `error_response_matches_v1_wire_format` | Failure uses `status: "error"` with the shared code/message error object; absent optional details are omitted; it round-trips |
| `event_matches_v1_wire_format` | An event has its name and data without a request ID; it round-trips |
| `response_cannot_contain_result_and_error` | A response containing both success and error payloads fails deserialization |
| `request_params_must_be_an_object` | Array-valued params fail deserialization |

**Limits:** The version-constant assertion does not test rejection of unsupported
versions by a running server. Present optional error details, the full error-code
vocabulary, request-ID numeric boundaries, and every malformed-envelope combination
are not covered by this suite.

## JSONL framing

**Source:** [framing.rs](../../crates/rigctld/src/ipc/framing.rs), internal test module

```bash
cargo test -p rigctld --lib ipc::framing::tests
```

The framing layer separates newline-delimited messages before JSON parsing. These
six tests use in-memory byte slices and buffered readers, so no socket is needed.

| Test | Behavior verified |
| --- | --- |
| `reads_complete_frame` | A newline ends a frame and is excluded from the returned bytes |
| `reconstructs_frame_across_multiple_reads` | A small reader buffer can split a message into chunks without changing its contents |
| `preserves_multiple_frames` | Two consecutive messages are returned separately and in order |
| `rejects_oversized_frame` | A frame with more than 1 MiB of payload returns `MessageTooLarge` |
| `rejects_partial_frame_at_eof` | EOF after partial message data returns `UnexpectedEof` |
| `clean_eof_returns_none` | EOF before any message bytes returns no frame rather than an error |

**Limits:** Oversize rejection is tested at the framing helper. This suite does
not demonstrate isolation of an oversized client in the production daemon. Exact
limit acceptance and all combinations of buffer boundaries are not tested.

## Request decoding

**Source:** [codec.rs](../../crates/rigctld/src/ipc/codec.rs), internal test module

```bash
cargo test -p rigctld --lib ipc::codec::tests
```

These four tests pass in-memory JSONL input through the request reader.

| Test | Behavior verified |
| --- | --- |
| `parses_valid_request` | A valid request yields the expected version, ID, method, and DPI parameter |
| `rejects_malformed_json` | Incomplete JSON produces an `InvalidJson` decoding error |
| `rejects_wrong_message_type` | An event-tagged envelope cannot deserialize as a request |
| `clean_disconnect_returns_none` | Empty input yields no request rather than an error |

**Limits:** Decoding does not dispatch methods or validate their parameters against
hardware capabilities. The response writer has no dedicated unit tests for its
serialization, write-error, or oversize-response branches; a successful response
write is exercised by the transport integration suite.

## Unix socket transport

**Source:** [ipc_server.rs](../../crates/rigctld/tests/ipc_server.rs)

```bash
cargo test -p rigctld --test ipc_server
```

Each test creates a temporary directory and binds `IpcServer` directly. Client
connections use Tokio's `UnixStream`. Test-owned tasks explicitly accept
connections, read requests, and write responses as required by each scenario.

| Test | Behavior verified |
| --- | --- |
| `binds_socket_with_expected_permissions` | Runtime directory permissions are `0700` and socket permissions are `0600` |
| `accepts_unix_socket_connection` | A client can connect and the server can accept its stream |
| `receives_request_over_unix_socket` | A JSONL request crosses the socket with the expected ID, method, and params |
| `persistent_connection_receives_multiple_requests` | One connection carries two requests without losing their IDs or method names |
| `sends_response_over_unix_socket` | A test-generated success response crosses the socket and preserves request ID `42` |
| `malformed_client_does_not_break_listener` | After the test drops a malformed client's connection, the same listener accepts a valid client's request |

**Limits:** These tests do not use the `rigctl::client::IpcClient` implementation.
They do not test out-of-order responses, event interleaving, concurrent request
dispatch, or oversized-message isolation over sockets. The malformed-client test
explicitly handles the read error and drops the bad connection; it does not prove
that a production accept loop implements that policy. A fixture method named
`system.status` is only test data and does not establish a real status API.

Socket lifetime and pathname cleanup are documented separately in the
[daemon lifecycle suite](./daemon-lifecycle.md).
