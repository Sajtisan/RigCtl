# RigCtl developer guide

## Purpose and current status

This guide is the starting point for building RigCtl, finding the right code, and
checking a change. Keeping these steps reproducible makes it easier to return to
the project or contribute without rediscovering its structure.

The current working tree includes the issue #4 daemon lifecycle implementation.
That describes local code, not a released or merged version. The workspace,
protocol types, transport components, and daemon lifecycle exist. Command parsing,
production request dispatch, configuration/state handling, logging integration,
hardware backends, and service packaging are unfinished.

## Development environment

- Use Linux; the initial target is Omarchy / Arch Linux.
- Install Rust and Cargo with support for Rust edition 2024 and Cargo resolver 3.
  The repository currently sets neither an explicit minimum Rust version nor a
  pinned toolchain file. Record a compatibility decision before introducing either.
- Have a working native linker and normal build tools for your Rust installation.
- Use a normal user account with a writable temporary directory and permission to
  create Unix sockets. Root is not required.
- Cargo needs dependency registry access on the first build unless dependencies
  are already cached. Hardware services are not needed for Foundation work.

From the repository root, inspect the toolchain and build using the workspace's
lockfile, including any local changes under review:

```bash
rustc --version
cargo --version
cargo check --workspace --locked
cargo build --workspace --locked
```

`--locked` prevents Cargo from changing dependency resolution. If a change
intentionally adds a dependency or feature requiring lockfile updates, regenerate
the lockfile as part of that change and review the resulting diff.

## Run the current daemon

In a desktop session with `XDG_RUNTIME_DIR` set:

```bash
cargo run -p rigctld
```

The daemon binds `$XDG_RUNTIME_DIR/rigctl/rigctld.sock` and waits. Press Ctrl+C to
send SIGINT and shut it down. Sending SIGTERM directly to the daemon has the same
orderly cleanup behavior. Exit status `0` means successful shutdown; failures are
printed to stderr and return `1`.

For an isolated run without using the session's RigCtl socket:

```bash
rigctl_dev_runtime=$(mktemp -d /tmp/rigctl-dev.XXXXXX)
XDG_RUNTIME_DIR="$rigctl_dev_runtime" cargo run -p rigctld
```

After shutdown, the socket is gone but the temporary base and `rigctl/` directory
remain. Inspect them if needed; this example does not automatically delete them.
The temporary override applies to that command only.

The listener does not yet accept or dispatch requests. A socket file existing
does not mean commands can be served. Likewise, `cargo run -p rigctl` currently
prints `rigctl`; the proposed CLI command hierarchy is not implemented.

## Find the right code

| Area | Location and responsibility |
| --- | --- |
| CLI executable | [rigctl main](../crates/rigctl/src/main.rs); currently a placeholder |
| Client transport | [client.rs](../crates/rigctl/src/client.rs); connecting, sending requests, receiving responses and events |
| Daemon executable | [rigctld main](../crates/rigctld/src/main.rs); Tokio runtime and process exit handling |
| Daemon lifecycle | [app.rs](../crates/rigctld/src/app.rs); startup stages, server ownership, and shutdown |
| Server transport | [ipc directory](../crates/rigctld/src/ipc/); framing, decoding, listening, connections, and owned-socket cleanup |
| Shared contracts | [rigctl-ipc](../crates/rigctl-ipc/src/); message shapes and the public error object |
| Automated tests | [test guide](./tests/README.md); suite layout, commands, and coverage limits |
| Generated files | `target/`; Cargo binaries and build caches, excluded from Git |

The CLI and daemon both depend on `rigctl-ipc`. The shared crate contains data
contracts, not socket, configuration, or hardware behavior. Future subsystem
implementations belong inside `rigctld` initially. See the
[workspace architecture](./Technical_Stack_RigCtl.md#rust-workspace-structure).

Startup integration functions currently perform no work for logging,
configuration, persistent state, or hardware. When implementing one of those
stages, introduce its error handling and owned resources deliberately; a function
name alone does not mean the integration exists.

## Validate and review a change

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git diff --check
git status --short
```

Use `cargo fmt --all` when intentionally formatting changed Rust code. Consult
the [suite guide](./tests/README.md) for focused test commands and sandbox/socket
requirements. Behavioral changes should have tests that exercise their observable
contract, including relevant failure paths.

Review both tracked and new files before committing; `git diff` alone omits the
contents of untracked files. Update the appropriate reference document and the
[decision log](./Development_Decisions_RigCtl.md) when a significant choice changes.
Committing and publishing are separate actions from implementation and validation.

## Where documentation belongs

| Information | Home |
| --- | --- |
| Product scope and direction | [Project Plan](./Project_Plan_RigCtl.md) |
| Architecture, ownership, and selected technology | [Technical Stack](./Technical_Stack_RigCtl.md) |
| Wire protocol and future method contracts | [IPC reference](./IPC_RigCtl.md) |
| Reasons and tradeoffs behind significant decisions | [Development Decisions](./Development_Decisions_RigCtl.md) |
| Experiments, evidence, and unverified device behavior | [Hardware research](./hardware/README.md) |
| Symptoms, diagnosis, and recovery | [Troubleshooting](./Troubleshooting_RigCtl.md) |

## Sections to grow as implementation progresses

These are documentation work items, not implemented features:

- **Subsystem development:** Add backend setup, interface boundaries, mocks, and
  hardware validation procedures when the first backend is introduced.
- **Configuration development:** Add schemas, isolated fixture examples, migration
  behavior, and safe persistence procedures when those implementations exist.
- **Service development:** Add the actual unit installation path, executable-path
  requirements, start/stop commands, and journal checks after a unit is added.
- **Dependency and toolchain policy:** Record a minimum toolchain and dependency
  update procedure when chosen; no version is selected by this skeleton.
- **Packaging and release checks:** Link the future package build, installation,
  upgrade, and release-validation procedures once they are reproducible.
