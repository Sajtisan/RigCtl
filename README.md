# RigCtl

**RigCtl** is a lightweight, CLI-first Linux hardware and system-control project written primarily in Rust.

It is designed around a single user-level daemon (`rigctld`) and a small command-line client (`rigctl`), with Linux-native integrations for hardware control, configuration, logging, and IPC.

> **Status:** Early development. The architecture and project plan are defined, while the initial implementation is being built out. Hardware support listed below is planned unless explicitly marked otherwise.

## Goals

RigCtl focuses on:

- low resource usage;
- one lightweight background daemon;
- direct Linux-native hardware integration;
- predictable, versioned IPC;
- strict and safe configuration handling;
- hardware capability validation before writes;
- reusable protocol work where it is useful to the wider Linux community.

RigCtl is **not** intended to be a generic plugin or module framework.

## Planned initial support

| Subsystem | Target | Backend / approach |
| --- | --- | --- |
| Mouse | Razer Viper V3 HyperSpeed | OpenRazer over D-Bus |
| Keyboard | Ajazz AK820 MAX | USB / HID, including protocol reverse engineering |
| Audio EQ | Linux audio output | PipeWire parametric EQ |
| Headset control | Skullcandy Crusher ANC 2 | Noise-control protocol research and implementation |

Planned Crusher ANC 2 noise-control states are:

- ANC
- Off
- Stay-Aware

Fan control through Linux `hwmon` / sysfs is part of the longer-term roadmap, but is intentionally deferred until the initial daemon, IPC, configuration, mouse, keyboard, and audio foundations are stable.

## Architecture

```text
                         rigctl
                           CLI
                            │
                            │ Unix socket + JSONL
                            ▼
                         rigctld
                          Daemon
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
           Mouse         Keyboard         Audio
```

Backend integrations are planned as:

```text
Mouse       → OpenRazer / D-Bus
Keyboard    → USB / HID
Audio EQ    → PipeWire / parametric EQ
Crusher     → headset control protocol
Fans        → hwmon / sysfs (later)
```

RigCtl's internal IPC uses a Unix domain stream socket with UTF-8 newline-delimited JSON messages.

The initial Rust workspace is intentionally small:

```text
crates/
├── rigctl/       # CLI client
├── rigctld/      # daemon and subsystem implementations
└── rigctl-ipc/   # shared IPC data contracts
```

The CLI does not contain hardware logic. Configuration, state, hardware validation, subsystem behavior, and IPC serving belong to `rigctld`.

## Linux integration

The initial target environment is **Omarchy / Arch Linux**.

Planned Linux integration includes:

- `systemd --user` for `rigctld`;
- `journald` for daemon logging;
- XDG Base Directory conventions;
- D-Bus for OpenRazer;
- USB/HID for keyboard communication;
- PipeWire for audio EQ;
- udev where device permissions require it.

The daemon is intended to run as a normal user process. Future fan-control requirements must not force the entire daemon to run as root.

## Configuration

RigCtl uses TOML with schema versioning.

```text
$XDG_CONFIG_HOME/rigctl/config.toml
$XDG_CONFIG_HOME/rigctl/profiles/mouse/
$XDG_CONFIG_HOME/rigctl/profiles/keyboard/
$XDG_CONFIG_HOME/rigctl/profiles/audio/
```

Persistent daemon state is kept separately under:

```text
$XDG_STATE_HOME/rigctl/
```

Transient runtime files, including the IPC socket, live under:

```text
$XDG_RUNTIME_DIR/rigctl/
```

`rigctld` is the authoritative reader and writer for configuration and profiles.

## Roadmap

The current development milestones are:

- **v0.1 — Foundation**  
  Rust workspace, IPC contracts and transport, daemon lifecycle, shared errors, XDG configuration infrastructure, logging, systemd integration, and automated test foundations.

- **v0.2 — Mouse**  
  Razer Viper V3 HyperSpeed support through OpenRazer, including verified DPI, polling-rate, battery, and named RigCtl profiles.

- **v0.3 — Keyboard**  
  Ajazz AK820 MAX protocol research and Linux control for verified features.

- **v0.4 — Audio**  
  PipeWire parametric EQ and Skullcandy Crusher ANC 2 noise-control integration.

- **v1.0 — Initial Release**  
  Cross-subsystem integration, packaging, documentation, permissions, testing, hardening, and release validation.

Fan control and Quickshell integration are planned for later work.

## Documentation

The README is intentionally kept high-level. The full architecture, protocol decisions, configuration rules, subsystem design, and development plan are documented here:

- [Technical Stack](./Docs/Technical_Stack_RigCtl.md)
- [Project Plan](./Docs/Project_Plan_RigCtl.md)

These documents are the main source of truth while RigCtl is under active development.

## Development

RigCtl is currently under active development. Installation and end-user usage instructions will be added once the relevant Foundation and subsystem work is implemented.

The project is designed so most daemon, IPC, configuration, and error-handling behavior can be tested without physical hardware. Hardware-in-the-loop testing is reserved for behavior that genuinely requires the target devices.

## License

MIT
