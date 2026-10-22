# Daemon lifecycle and process tests

[Back to the test guide](./README.md)

These suites cover the issue #4 lifecycle implementation. The daemon binds its
listener and waits for shutdown; it does not yet accept or dispatch commands.
The rationale for the boundaries is recorded in
[D001–D007](../Development_Decisions_RigCtl.md#decision-index).

## Application lifecycle

**Source:** [app_lifecycle.rs](../../crates/rigctld/tests/app_lifecycle.rs)

```bash
cargo test -p rigctld --test app_lifecycle
```

These twelve integration tests call the public `Application` API directly within
a Tokio test runtime. Each test uses `tempfile::tempdir()` and supplies the RigCtl
runtime directory explicitly, so it does not rely on the developer's XDG settings.
`run_until()` accepts a shutdown future, letting tests control when the application
ends without registering real process signals.

| Test | Scenario and expected result |
| --- | --- |
| `waits_for_shutdown_without_hardware_and_cleans_up_for_restart` | Start without hardware, confirm the task is waiting, connect an idle client, and trigger shutdown; the task succeeds, the socket disappears, the directory remains, and a second startup can bind the same path |
| `failed_shutdown_future_still_cleans_up` | Inject a shutdown error; return that error while still removing the socket |
| `dropping_application_cleans_up` | Drop the application without running it; best-effort cleanup removes its socket |
| `cancelling_running_application_cleans_up` | Abort a task waiting on a never-completing shutdown future; the task reports cancellation and the socket is removed |
| `occupied_socket_is_preserved_and_original_listener_still_works` | Start a second application at an active socket path; startup fails and the first listener still permits a connection |
| `occupied_regular_file_is_preserved` | Put a normal file at the socket path; startup fails without changing the file contents |
| `cleanup_succeeds_if_socket_was_already_removed` | Remove the pathname before shutdown; cleanup treats its absence as success |
| `cleanup_preserves_replacement_file` | Move the original socket and replace its pathname with a regular file; shutdown preserves the replacement |
| `cleanup_preserves_replacement_socket` | Move the original socket and start a replacement listener at its old path; shutting down the original preserves the replacement's connectivity |
| `cleanup_preserves_replacement_symlink` | Move the socket and place a symlink at the old path; shutdown preserves both the symlink and its target |
| `explicit_cleanup_failure_is_reported` | Move the runtime directory and replace its old name with a regular file; cleanup returns contextual failure and preserves the blocking file |
| `invalid_runtime_directory_fails_startup` | Use a regular file as the runtime-directory path; startup returns contextual failure without altering its contents |

### Why the filesystem scenarios are arranged this way

Replacement tests rename the original socket rather than immediately deleting it.
This retains its original filesystem inode and avoids relying on whether the
filesystem immediately reuses an inode for a new object. The tests can then check
the ownership guard's behavior against a distinct replacement.

The explicit-failure test blocks access to the original socket path by turning its
parent path into a regular file. It produces a deterministic cleanup error without
depending on permission checks that a privileged process could bypass.

### Limits

- The idle client connects to the operating system's listener backlog. It is not
  an accepted connection running a request handler, and no response is expected.
- Lifecycle tests demonstrate startup with no implemented hardware hooks. They do
  not demonstrate recovery from an unavailable real backend or initialization
  failures inside future configuration/state/subsystem implementations.
- Cleanup tests cover controlled pathname replacement. They do not prove an atomic
  ownership check and unlink under concurrent filesystem mutation.
- SIGKILL, process crashes, and stale-socket recovery are not covered. They cannot
  be inferred from tests of ordinary resource drops or Tokio task cancellation.
- Signal-registration failure and the socket-permission-setting failure path have
  no dedicated injected-failure tests. Combined shutdown and cleanup failure
  precedence is also not directly tested.

## Executable behavior

**Source:** [daemon_process.rs](../../crates/rigctld/tests/daemon_process.rs)

```bash
cargo test -p rigctld --test daemon_process
```

These four tests launch the actual daemon binary using Cargo's
`CARGO_BIN_EXE_rigctld` path. They do not require a separately installed executable.
The test helper captures stdout/stderr and gives the child temporary XDG runtime,
configuration, and state paths. Startup readiness is observed by waiting for the
socket pathname while checking that the child remains alive.

| Test | Scenario and expected result |
| --- | --- |
| `sigint_shuts_down_cleanly` | Send SIGINT after socket creation; the daemon exits successfully, removes the socket, retains its runtime directory, emits no stdout/stderr, and creates no configuration or state directory |
| `sigterm_shuts_down_cleanly` | Verify the same behavior using SIGTERM, the shutdown signal relevant to the future user service |
| `missing_runtime_environment_exits_with_failure` | Remove `XDG_RUNTIME_DIR` from the child environment; exit status is `1`, stdout is empty, and stderr identifies the missing variable |
| `occupied_socket_path_exits_with_failure_without_deleting_file` | Prepopulate the socket pathname with a regular file; exit status is `1`, stdout is empty, stderr identifies IPC startup failure, and the file contents are preserved |

### Synchronization and cleanup

The process helper uses five-second deadlines for readiness and exit waits, with
ten-millisecond sleeps between checks. It sends signals through the external
`kill` utility using only the child PID. Its drop guard attempts to kill and reap
the child even if the test fails, so a failed assertion does not normally leave a
daemon process behind.

Readiness here means that the socket has been created; it does not promise request
service readiness. Output is read after the process exits. The current skeleton
emits no normal output, so these tests are not stress tests of large diagnostic
streams.

### Limits

These tests verify the executable directly, not a systemd unit, installation flow,
or journald integration. They do not verify root refusal, a systemd restart policy,
or behavior after fatal signals. No test requires root privileges. The startup
failure scenarios verify exit code `1`; CLI-specific codes and JSON rendering
remain outside these suites.
