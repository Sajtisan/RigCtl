# RigCtl test documentation

This directory explains the automated tests currently present in the repository.
The [Technical Stack testing strategy](../Technical_Stack_RigCtl.md#testing-strategy)
also describes future coverage; this guide documents implemented tests only.

## Running the tests

Run commands from the repository root:

```bash
cargo test --workspace
```

The suite needs a Rust toolchain supporting the workspace's Rust 2024 edition,
Linux/Unix socket and signal support, and a writable temporary directory. Process
tests also invoke the external `kill` executable through `PATH` to signal their
own child processes. Cargo may need registry access to download dependencies on
the first run.

No target hardware, OpenRazer daemon, PipeWire instance, installed RigCtl service,
or running RigCtl daemon is needed. Filesystem tests use temporary directories.
Process tests supply isolated XDG paths to their child process; they do not change
the test runner's XDG environment or use the developer's RigCtl configuration.

Useful commands:

| Command | Purpose |
| --- | --- |
| `cargo test --workspace -- --list` | List tests without executing their bodies |
| `cargo test -p rigctl-ipc --test protocol_v1` | Run the shared wire-contract suite |
| `cargo test -p rigctld --lib` | Run framing and decoder unit tests |
| `cargo test -p rigctld --test ipc_server` | Run Unix socket transport tests |
| `cargo test -p rigctld --test app_lifecycle` | Run application lifecycle tests |
| `cargo test -p rigctld --test daemon_process` | Run executable and signal tests |

To inspect one failure with its output and a backtrace:

```bash
RUST_BACKTRACE=1 cargo test -p rigctld --test app_lifecycle cleanup_preserves_replacement_socket -- --exact --nocapture
```

Code-quality checks are separate from behavioral tests:

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Suite map

Inventory recorded on 2026-10-22, including the local issue #4 implementation:

| Suite | Count | Test level | Documentation |
| --- | --- | --- | --- |
| `rigctl-ipc/tests/protocol_v1.rs` | 7 | Public data-contract integration tests | [Protocol and transport](./ipc.md#protocol-contracts) |
| `rigctld/src/ipc/framing.rs` | 6 | Unit tests of private framing behavior | [Protocol and transport](./ipc.md#jsonl-framing) |
| `rigctld/src/ipc/codec.rs` | 4 | Unit tests of request decoding | [Protocol and transport](./ipc.md#request-decoding) |
| `rigctld/tests/ipc_server.rs` | 6 | Transport integration tests using real temporary sockets | [Protocol and transport](./ipc.md#unix-socket-transport) |
| `rigctld/tests/app_lifecycle.rs` | 12 | Library lifecycle integration tests | [Daemon lifecycle](./daemon-lifecycle.md#application-lifecycle) |
| `rigctld/tests/daemon_process.rs` | 4 | Child-process integration tests | [Daemon lifecycle](./daemon-lifecycle.md#executable-behavior) |
| **Total** | **39** | | |

The `rigctl` client crate currently contains no automated tests. Cargo may also
report successful targets with zero unit tests or documentation tests; these are
not additional coverage.

## How to interpret coverage

The protocol suite verifies serialized shapes without opening sockets. Framing and
decoder tests exercise in-memory input. Transport tests drive `IpcServer` directly
and supply their own request handling. Lifecycle tests own an `Application`, and
process tests execute the actual `rigctld` binary built by Cargo.

The production daemon skeleton binds a listener but does not accept or dispatch
requests. A passing transport test is therefore not an end-to-end CLI command
test. Strings such as `mouse.dpi.get` in fixtures do not cause hardware access.

Current coverage gaps include client-side response correlation and interleaved
events, end-to-end CLI output and exit behavior, request dispatch and version-error
responses, event subscriptions, configuration/state behavior, logging, and hardware
backends. There are no hardware-in-the-loop suites yet. More specific limitations
appear in the suite guides.

## Test conventions

- Unit tests for private helpers live beside the implementation under `#[cfg(test)]`.
  Public API and process scenarios live in each crate's `tests/` directory.
- Use temporary paths for filesystem scenarios. Pass runtime paths directly to
  the application where possible; supply environment overrides only to child
  processes when testing production startup.
- Coordinate asynchronous lifecycle tests with channels. The current lifecycle
  suite uses five-second timeouts around readiness and shutdown task waits.
- Process helpers check readiness and exit against five-second deadlines, sleeping
  briefly between checks. Their guard kills and reaps the child on drop, including
  assertion failures. These polling helpers belong to the tests; the production
  daemon waits asynchronously.
- Keep physical hardware tests separate when introduced. Document their required
  devices and explicit execution procedure rather than adding them to the normal
  hardware-independent suite.
- Update the inventory and scenario tables when adding, removing, or changing a
  test. Record architectural testing decisions in the
  [development decision log](../Development_Decisions_RigCtl.md).

## Diagnosing failures

| Symptom | What to check |
| --- | --- |
| `Operation not permitted` or `PermissionDenied` while binding test sockets | Whether the execution sandbox permits Unix socket operations; these tests need socket access, not root privileges |
| Failure to execute `kill` | Whether an external `kill` executable is available on `PATH`; a shell builtin alone is insufficient |
| Unix socket path is too long | Whether `TMPDIR` points to a deeply nested directory; retry with a short temporary base such as `/tmp` |
| `daemon did not bind its socket` or `daemon did not exit` | The process test's five-second deadline expired; rerun the individual test with output and a backtrace to investigate |
| A test name or documented count differs from Cargo's list | The documentation needs to be updated alongside the tests |
