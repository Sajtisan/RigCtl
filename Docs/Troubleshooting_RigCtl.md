# RigCtl troubleshooting

## Purpose and current limits

This guide connects an observable symptom to evidence and an appropriate recovery
step. It should prevent repeated debugging work and provide a consistent place to
record fixes as more features become usable.

Today it covers building and running the daemon lifecycle skeleton. Request
dispatch, hardware backends, configuration parsing, logging initialization, and a
systemd service unit are not implemented in the current working tree. Examples of
future commands elsewhere in the project documents do not establish working CLI
features. See the [developer guide](./Development_RigCtl.md) for startup and checks.

## Collect basic evidence

Run from the repository root:

```bash
git status --short
git branch --show-current
git log -1 --oneline
rustc --version
cargo --version
printenv XDG_RUNTIME_DIR
```

Record the exact command, its exit status, and stderr. Include whether the problem
occurs in a terminal, test runner, sandbox, or eventually a user service. For a
build or test failure, include the first relevant error rather than only the final
summary. Describe local changes because the commit ID alone does not identify an
uncommitted working tree.

## Current symptoms

### The daemon reports that XDG_RUNTIME_DIR is not set

**Meaning:** Default socket-path resolution has no runtime base. The daemon exits
with status `1` before a usable listener is created.

**Check:** Inspect `printenv XDG_RUNTIME_DIR` in the same environment that launches
the daemon. A desktop terminal and another launcher may have different environments.

**Recovery:** Run from the normal user session, or use the command-scoped temporary
runtime example in the [developer guide](./Development_RigCtl.md#run-the-current-daemon).
The current daemon does not invent a fallback runtime path.

### Binding fails with an occupied path or address-in-use error

**Meaning:** Something already exists at
`$XDG_RUNTIME_DIR/rigctl/rigctld.sock`. It may be another instance, a stale socket,
or a different filesystem object. Startup does not delete it automatically.

**Check:** Identify any existing daemon process and inspect the pathname. When
`XDG_RUNTIME_DIR` is set, these commands inspect rather than modify the object:

```bash
pgrep -a -x rigctld
ls -ld -- "$XDG_RUNTIME_DIR/rigctl"
ls -l -- "$XDG_RUNTIME_DIR/rigctl/rigctld.sock"
```

**Recovery:** If it belongs to a live instance, stop that instance normally before
starting another. For investigation, an isolated temporary runtime avoids altering
the existing path. A leftover object should only be removed manually after its
ownership and the absence of a live owner are established; there is no automatic
stale-socket recovery procedure yet.

### Startup or tests fail with PermissionDenied or Operation not permitted

**Meaning:** The failure can come from directory permissions or an execution
environment that prohibits Unix socket operations.

**Check:** Inspect the runtime directory and the command's sandbox restrictions.
The expected RigCtl directory mode is `0700`; the socket mode is `0600`. The normal
automated socket tests also need permission to create temporary Unix sockets.

**Recovery:** Use a directory owned by the current user and an execution
environment that permits the required socket operations. Elevating the entire
daemon is not part of the initial design. For failures confined to tests, see
[test diagnostics](./tests/README.md#diagnosing-failures).

### The daemon appears idle, with no startup output

**Meaning:** The current skeleton emits no success message or normal logs. After
binding its socket, it waits asynchronously for SIGINT or SIGTERM.

**Check:** Confirm that the process and socket exist. Normal logging integration
is still deferred. Ctrl+C should trigger orderly shutdown and remove the socket.

**Recovery:** No action is needed merely because the process is quiet. If startup
fails, inspect stderr and the exit status instead.

### A client connects but gets no response, or a planned command does nothing useful

**Meaning:** The lifecycle skeleton binds a listener but does not accept or
dispatch requests. The operating system may complete a connection into the backlog.
The current `rigctl` executable only prints its name.

**Check:** Compare the behavior against the implementation-status section of the
[IPC reference](./IPC_RigCtl.md#implementation-status).

**Recovery:** Use the component tests for transport validation. Production command
handling requires future implementation; changing hardware settings or retrying
against the same listener will not supply it.

### A socket remains after SIGKILL or a crash

**Meaning:** Forced termination can bypass Rust cleanup. Closing the process's
socket descriptor does not necessarily remove the filesystem pathname.

**Check:** Distinguish forced termination from SIGINT/SIGTERM and establish whether
any process still owns the endpoint. A subsequent bind failure does not by itself
prove the old object is safe to delete.

**Recovery:** Follow the occupied-path investigation above. Use SIGINT or SIGTERM
for normal stops. Automatic crash recovery is deferred.

### Shutdown reports a socket cleanup failure

**Meaning:** The listener has been closed, but the owned pathname could not be
inspected or removed. The daemon reports failure with exit status `1`.

**Check:** Inspect the error context and whether a parent directory was moved,
replaced, or became inaccessible. The cleanup code preserves replacement objects
whose filesystem identity does not match the original socket.

**Recovery:** Investigate the specific filesystem error and remaining object before
restarting. Do not assume cleanup failure means the daemon is still running.

## Future troubleshooting sections

These sections intentionally have no device-specific fixes yet. Fill them only
after the relevant behavior and diagnostics can be reproduced.

| Area | Why the section matters | Add when implemented |
| --- | --- | --- |
| Configuration and profiles | Separate syntax/schema failures from missing references and persistence errors | Example diagnostic, affected file, validation command, safe correction, recovery behavior |
| systemd and journald | Distinguish binary startup problems from service environment or lifecycle policy | Actual unit name/install path, status commands, log commands, expected output, start/stop recovery |
| OpenRazer mouse | Separate missing hardware, backend availability, capabilities, and read-back mismatch | Verified discovery checks, backend/driver versions, permission diagnosis, operation-specific recovery |
| Ajazz keyboard | Diagnose interface selection, report errors, and connection-mode differences | Verified device IDs, HID reports, permitted transports, protocol fixtures, recovery limitations |
| PipeWire EQ | Identify the correct output and distinguish filter setup from profile state | Selected integration mechanism, graph inspection, reconnect behavior, safe bypass/removal |
| Crusher ANC 2 | Separate unknown current mode from control-transport failures | Verified channel, notifications/query behavior, reconnect checks, concurrent controller behavior |
| Fans, later | Diagnose permissions and missing sensors without losing safe control | Verified target interface, selected privilege model, fallback state, recovery and stop procedures |

## Template for a new troubleshooting entry

```text
Symptom:
Applies to: version/commit, backend, device, environment
Expected behavior:
Evidence to collect:
Read-only diagnostic commands and example output:
Confirmed cause and how it was established:
Recovery steps and any state they change:
How to confirm recovery:
Known limits or unresolved causes:
Related issue, test, research note, or decision:
```

When a diagnosis reveals a new design constraint, also update the
[decision log](./Development_Decisions_RigCtl.md) or the relevant
[hardware research note](./hardware/README.md).
