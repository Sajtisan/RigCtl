# RigCtl — Development Decisions

This document records significant development choices and why they were made.
The [Project Plan](./Project_Plan_RigCtl.md) describes the intended product;
the [Technical Stack](./Technical_Stack_RigCtl.md) describes its architecture and
contracts. This log preserves the reasoning behind those choices and later changes.

An **accepted** decision is a chosen direction, not evidence that its implementation
has been committed, merged, or released. Consult the linked issue and Git history
for implementation status.

## Maintaining the log

- Add an entry when a change affects architecture, scope, public behavior,
  dependencies, resource ownership, or testing strategy. Routine fixes do not need
  an entry unless they reveal a new constraint or change an earlier decision.
- Use the next stable ID and record the date, status, context, decision, reasoning,
  consequences, and relevant issue or code links.
- Use **Proposed**, **Accepted**, or **Superseded** for decision status. Keep open
  questions in the separate section below until a choice is made.
- Preserve historical entries. If a decision changes, add a new entry and mark the
  old one superseded with a link to its replacement. Correct factual mistakes in
  place and identify substantive corrections.
- Update the Project Plan and Technical Stack when a decision changes the current
  design. The log explains the history; it does not replace those specifications.

## Existing architectural baseline

These choices already appear in the project documents. They are summarized here
for orientation; their original decision dates are not established by this log.
This is not a complete retrospective of every earlier development choice.

| Existing choice | Practical consequence | Source |
| --- | --- | --- |
| One user-level `rigctld`, with `rigctl` and a future Quickshell UI as clients | Hardware and application state belong to the daemon; systemd owns its process lifecycle | [Project architecture](./Project_Plan_RigCtl.md#architecture) |
| Three first-party crates: `rigctl`, `rigctld`, and `rigctl-ipc` | Both executables share data contracts; hardware and socket behavior stay outside the shared crate | [Workspace structure](./Technical_Stack_RigCtl.md#rust-workspace-structure) |
| Unix stream socket with versioned JSONL messages | CLI and future persistent clients share one local protocol | [IPC](./Project_Plan_RigCtl.md#ipc) |
| Daemon-owned TOML configuration and profiles, separate from persistent state | Clients request changes over IPC; defaults and last-applied state have distinct meanings | [Configuration](./Project_Plan_RigCtl.md#configuration) |
| Capability validation before hardware writes, with verification where supported | An invalid complete profile causes zero hardware writes; partial application remains failure | [Mouse hardware safety](./Project_Plan_RigCtl.md#mouse-hardware-safety) |
| Structured tracing to journald and one public error vocabulary | Diagnostic detail stays separate from the stable client-facing error contract | [Logging and errors](./Project_Plan_RigCtl.md#logging-diagnostics-and-shared-errors) |
| Mouse, keyboard, and audio are initial subsystems; fans and Quickshell come later | No plugin framework or privileged fan architecture is introduced into the foundation | [Project overview](./Project_Plan_RigCtl.md#overview) |

## Decision index

Entries D001–D007 were recorded on **2026-10-22** for
[#4 — Daemon startup skeleton](https://github.com/Sajtisan/RigCtl/issues/4).
Later entries identify their own scope.

| ID | Decision | Status |
| --- | --- | --- |
| [D001](#d001-library-owned-lifecycle) | Library-owned lifecycle with a thin executable | Accepted |
| [D002](#d002-bind-the-listener-without-request-dispatch) | Bind the listener without request dispatch | Accepted |
| [D003](#d003-explicit-stages-for-unfinished-components) | Explicit stages for unfinished components | Accepted |
| [D004](#d004-asynchronous-waiting-and-process-signals) | Asynchronous waiting and process signals | Accepted |
| [D005](#d005-inject-paths-and-shutdown-for-tests) | Inject paths and shutdown for tests | Accepted |
| [D006](#d006-own-and-clean-up-the-socket) | Own and clean up the socket | Accepted |
| [D007](#d007-report-lifecycle-failures-without-expanding-error-policy) | Report lifecycle failures without expanding error policy | Accepted |
| [D008](#d008-focused-reference-documents-and-research-records) | Focused reference documents and research records | Accepted |

## D001 Library-owned lifecycle

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** The daemon previously printed its name and exited. The existing IPC
transport needed an application that could own it for the lifetime of the process.

**Decision:** Keep executable bootstrap and exit-code handling in `main.rs`.
Place startup, runtime ownership, and shutdown in the library's `app.rs`.
`Application` owns the IPC server; the public `run()` function connects the
application to process shutdown signals.

**Reasoning:** Lifecycle behavior can be exercised directly in tests without
launching the production executable for every scenario. Future configuration and
subsystem resources have a clear owner in the same daemon process.

**Consequences:** `run()` is now asynchronous and returns `io::Result<()>`.
The binary supplies the Tokio runtime and translates the result into a process
exit status. No additional daemon or crate is introduced.

**References:** [Application](../crates/rigctld/src/app.rs),
[executable entry point](../crates/rigctld/src/main.rs).

## D002 Bind the listener without request dispatch

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** We considered three scopes: binding only; accepting requests and
returning protocol errors; or adding a successful status method. Issue #4 specifies
the startup lifecycle and resource ownership, but does not explicitly require a
request-processing loop or a new method.

**Decision:** Reuse the existing transport to bind and retain the listener. Do not
accept or dispatch requests in this lifecycle implementation, and do not add a
status API.

**Reasoning:** This follows the agreed issue boundary and avoids defining new API
behavior before its state and subsystem models exist.

**Consequences:** A socket existing means the listener is bound; it does not mean
the daemon can serve commands yet. A client may connect through the operating
system's backlog but receives no response. Request dispatch requires follow-up
work. During this review, no dedicated dispatch issue existed in the inspected
backlog; this log does not assume one has subsequently been created.

**References:** [Issue #4](https://github.com/Sajtisan/RigCtl/issues/4),
[#3 transport scope](https://github.com/Sajtisan/RigCtl/issues/3).

## D003 Explicit stages for unfinished components

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** The lifecycle needs places for components whose actual implementations
belong to separate issues.

**Decision:** Call these stages in order: application initialization, logging,
configuration loading, persistent-state restoration, independent mouse/keyboard/
audio initialization, then IPC binding. Until their implementations exist, the
first stages are documented functions that perform no work.

**Reasoning:** Startup order and responsibility are visible without prematurely
implementing configuration, logging, or hardware protocols. Separate subsystem
entry points allow later initialization to remain independently organized.

**Consequences:** Names such as `load_configuration()` describe integration points,
not completed features. They currently read and write nothing, and startup requires
no physical hardware. These stages must become fallible and own their resources
when real implementations are added. No generic plugin or lifecycle framework is
introduced. Their source-code order does not decide which subsystem is developed
first.

**References:** [Startup stages](../crates/rigctld/src/app.rs),
[configuration parsing #7](https://github.com/Sajtisan/RigCtl/issues/7),
[logging #9](https://github.com/Sajtisan/RigCtl/issues/9).

## D004 Asynchronous waiting and process signals

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** The daemon should stay alive with little idle work and shut down both
from a terminal and when a future user service stops it.

**Decision:** Use a Tokio runtime with one execution thread for the binary. Register
SIGINT and SIGTERM handlers before binding the socket, then asynchronously wait
for either signal. Enable Tokio's signal feature rather than adding a separate
application-level signal library.

**Reasoning:** Ctrl+C sends SIGINT; service managers normally use SIGTERM to stop
processes. Registering first avoids exposing the listener before these shutdown
handlers exist. Asynchronous waiting requires no periodic polling loop, and the
skeleton does not need a pool of worker threads.

**Consequences:** Tokio adds its `signal-hook-registry` dependency to `Cargo.lock`.
Future blocking backend work must be kept off the runtime thread, or the runtime
choice revisited. SIGKILL cannot run cleanup; this implementation does not provide
automatic crash recovery.

**References:** [Signal handling](../crates/rigctld/src/app.rs),
[runtime entry point](../crates/rigctld/src/main.rs),
[daemon dependencies](../crates/rigctld/Cargo.toml).

## D005 Inject paths and shutdown for tests

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** Most lifecycle tests should not depend on the developer's session,
configuration, hardware, or process-wide signal handlers.

**Decision:** Provide `Application::start(runtime_dir)` and
`Application::run_until(shutdown)`. The explicit path identifies the RigCtl runtime
directory itself. The shutdown future returns `io::Result<()>`; tests can complete
it deterministically. Production uses `start_defaults()` and real Unix signals.

**Reasoning:** Temporary directories and injected shutdown isolate application
tests without mutating process-wide environment variables. A small set of child
process tests verifies the real executable, signals, stderr, and exit codes.

**Consequences:** Application tests use real temporary Unix sockets but no hardware.
Process tests set XDG variables only for their child process and use bounded waits.
Tokio's timing feature is enabled for tests. Socket tests need an execution
environment that permits Unix socket operations.

**References:** [Lifecycle tests](../crates/rigctld/tests/app_lifecycle.rs),
[process tests](../crates/rigctld/tests/daemon_process.rs).

## D006 Own and clean up the socket

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** Closing a Unix listener does not remove its filesystem pathname. A
leftover socket can prevent the next startup. Blindly deleting the pathname could
also remove a replacement object belonging to another instance.

**Decision:** Record the socket's filesystem device and inode after binding.
Explicit shutdown closes the listener, then checks the pathname with
`symlink_metadata` and removes it only if it is still a socket with that identity.
Successful cleanup disarms the ownership guard. Dropping the server also attempts
cleanup, but cannot report its errors. Retain the runtime directory.

**Reasoning:** The process should release the endpoint it created while preserving
replacement files, symlinks, and sockets. The ownership guard also covers task
cancellation, ordinary drops, and failure while setting socket permissions after
the guard has been created.

**Consequences:** An already-removed socket counts as cleaned up. Explicit cleanup
errors are returned; cleanup in `Drop` is best effort. Startup never automatically
deletes an occupied path, including a potentially stale socket. Device/inode checks
reduce accidental deletion but do not make the separate metadata check and unlink
atomic against concurrent filesystem changes. Stale-socket recovery and stronger
concurrent-path handling require separate design if needed.

**References:** [Socket ownership and shutdown](../crates/rigctld/src/ipc/server.rs),
[cleanup and replacement tests](../crates/rigctld/tests/app_lifecycle.rs).

## D007 Report lifecycle failures without expanding error policy

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Issue #4

**Context:** Startup and shutdown failures must reach the caller, while the complete
shared error policy belongs to issue #5.

**Decision:** Return internal `io::Result` values with operation context. The binary
prints a concise error to stderr and exits with status `1` on failure; orderly
shutdown returns `0`. It emits no success text to stdout. Reuse existing XDG runtime
resolution and report missing environment variables or bind failures.

**Reasoning:** This meets the lifecycle issue's failure-reporting requirement
without establishing competing public error codes or prematurely implementing the
CLI's error-rendering policy.

**Consequences:** The current skeleton is quiet and has no tracing subscriber yet.
Cleanup still runs when the injected shutdown future fails. If both shutdown and
cleanup fail, the shutdown error takes precedence in the returned result. Typed
public error mappings and structured daemon diagnostics remain separate work.

**References:** [Application error propagation](../crates/rigctld/src/app.rs),
[process exit handling](../crates/rigctld/src/main.rs),
[shared errors #5](https://github.com/Sajtisan/RigCtl/issues/5).

## D008 Focused reference documents and research records

**Recorded:** 2026-10-22 · **Status:** Accepted · **Scope:** Documentation organization

**Context:** Returning to the project requires setup instructions, a usable IPC
reference, research history, and diagnostic procedures. Some of those areas have
no implementation or experimental findings yet.

**Decision:** Add focused developer, IPC, hardware research, and troubleshooting
documents. Move the detailed IPC specification out of the Technical Stack into
its dedicated reference, retaining an overview and link. Give unfinished sections
a purpose, open questions, and an evidence/template structure instead of inventing
results. Link all guides from the README.

**Reasoning:** One detailed wire reference reduces specification drift. Separating
confirmed observations from hypotheses prevents planned hardware capabilities from
being mistaken for measured support. Purposeful skeletons show where later work
belongs without claiming that the feature exists.

**Consequences:** Future protocol changes belong in the IPC reference as well as
the relevant types and tests. Hardware findings remain research until supported
by evidence and adopted into the design. These documentation changes select no
subsystem priority, protocol method schema, hardware value range, or backend
mechanism. Existing technical sections outside the extracted IPC specification
remain authoritative for their topics.

**References:** [Developer guide](./Development_RigCtl.md),
[IPC reference](./IPC_RigCtl.md), [hardware research](./hardware/README.md),
[troubleshooting](./Troubleshooting_RigCtl.md).

## Open questions and deferred work

These are not accepted decisions:

- **Subsystem development order:** Mouse, keyboard, and audio implementation
  priority remains undecided. Existing roadmap orderings should be reconciled when
  a priority is chosen.
- **Request dispatch:** Define its follow-up scope and track it explicitly before
  adding handlers, status response fields, or event subscription behavior.
- **Crash recovery:** Decide whether and how a future startup can identify a stale
  socket without interfering with a live instance.
- **Backend execution and shutdown:** Actual hardware integrations will determine
  which work may block, how operations are cancelled or completed, and which
  resources must be released before socket cleanup.
- **Persistent-state schema and hardware-specific configuration:** These are still
  deferred in the main documents. Lifecycle placeholders do not settle their design.

## Reading the issue #4 implementation

The normal path is:

```text
main.rs starts Tokio
    -> run() registers SIGINT and SIGTERM
    -> Application runs the startup stages
    -> IpcServer binds the socket
    -> run_until() waits for shutdown
    -> IpcServer closes the listener and cleans up its socket
    -> main.rs returns exit status 0, or reports an error and returns 1
```

`Application` owning the server means that dropping the application drops the server
as well. The server's listener field closes first; its socket guard then attempts
to remove the owned pathname. Explicit shutdown uses the same cleanup logic but
can return an error to the caller. That is why both explicit shutdown and a `Drop`
fallback exist.

The `?` operator returns an error to the caller immediately when a step fails.
`.await` lets the asynchronous runtime wait for work, rather than repeatedly
checking in a busy loop. The future passed to `run_until()` represents the event
that ends the daemon's lifetime: a process signal in production, or a controlled
test event in lifecycle tests.
