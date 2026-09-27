# Project Plan — RigCtl

## Overview

**RigCtl** is a lightweight Linux hardware and system-control project designed primarily for an Omarchy / Arch Linux desktop setup.

The initial version focuses on three concrete subsystems:

- **Mouse Controller** — Razer Viper V3 HyperSpeed
- **Keyboard Controller** — Ajazz AK820 MAX
- **Audio** — PipeWire EQ profiles and Skullcandy Crusher ANC 2 noise control

The **Fan Controller** remains part of the longer-term roadmap, but it is explicitly deferred until after the initial version.

RigCtl is **not** intended to be a general-purpose plugin or module framework. Mouse, keyboard, and audio are part of the planned initial product scope.

The main goals are:

- minimal resource usage;
- direct Linux hardware integration;
- reliable control of the target hardware;
- reverse engineering unsupported hardware where necessary;
- CLI-first development;
- one lightweight Linux-native daemon;
- clean separation of device-specific protocol logic from presentation/UI code;
- later integration with Omarchy / Quickshell.

The CLI and daemon will be implemented first. A graphical interface is intentionally deferred and is expected to live in the Omarchy / Quickshell environment rather than in a browser or Electron application.

---

## Subject to Change

This plan may change as development progresses based on hardware limitations, reverse-engineering findings, testing, Linux integration requirements, resource usage, and changing priorities.

Whenever an architectural, scope, stack, or behavior decision is finalized, the affected Markdown documentation should be regenerated so that these documents remain the current source of truth.

---

## Architecture

RigCtl uses a CLI/client plus daemon model:

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

Backend connections are conceptually:

```text
Mouse       → openrazer-daemon / D-Bus
Keyboard    → USB / HID
Audio EQ    → PipeWire / parametric EQ
Crusher     → headset control protocol (undecided)
Fans        → hwmon / sysfs (later implementation)
```

`rigctld` owns the internal Unix-socket endpoint. The `rigctl` CLI and future Quickshell UI act as clients.

The future Quickshell interface should use the same backend:

```text
                    Quickshell UI
                         │
                         ▼
                      rigctld
                         ▲
                         │
                      rigctl
                        CLI
```

The UI is therefore a future client of RigCtl, not the center of the architecture.

---

## CLI and Daemon Model

### `rigctl`

`rigctl` is the user-facing command-line client.

Responsibilities:

- parse commands;
- validate basic user input;
- send requests to `rigctld`;
- display results and errors;
- provide a scriptable interface.

Example commands:

```bash
rigctl status
rigctl version

rigctl mouse status
rigctl mouse dpi get
rigctl mouse dpi set 800
rigctl mouse polling-rate get
rigctl mouse polling-rate set 1000
rigctl mouse profile list
rigctl mouse profile current
rigctl mouse profile apply gaming

rigctl keyboard status
rigctl keyboard keymap get
rigctl keyboard remap capslock escape
rigctl keyboard profile list
rigctl keyboard profile current
rigctl keyboard profile apply gaming

rigctl audio status
rigctl audio profile list
rigctl audio profile current
rigctl audio profile apply music
rigctl audio noise-control get
rigctl audio noise-control set anc
rigctl audio noise-control set off
rigctl audio noise-control set stay-aware
```

The public CLI will use an explicit hierarchy:

```text
rigctl [GLOBAL OPTIONS] <subsystem> <resource/action> ...
```

It will prefer explicit verbs such as `get`, `set`, `list`, `current`, `apply`, `create`, and `delete` rather than overloading positional values.

Normal output is concise and human-readable. Commands that return data will support `--json`; successful JSON output will represent the requested object directly rather than wrapping it in a redundant success envelope. Requested results go to stdout, while errors, warnings, and diagnostics go to stderr.

Initial global options are `--help`, `--version`, `--json`, and `--verbose`. Initial exit codes are:

```text
0   Success
1   Runtime or operation failure
2   Invalid CLI usage
3   rigctld unavailable
```

The CLI validates syntax, argument presence, primitive types, and universally invalid values. `rigctld` remains authoritative for actual device capabilities, supported values, backend availability, connection type, and current hardware state.

`rigctl` must not silently start the daemon. If the IPC socket is unavailable, it reports `daemon_unavailable` and exits with code 3; systemd owns the daemon lifecycle.

### `rigctld`

`rigctld` is the lightweight background daemon.

It will run as `rigctld.service` under `systemd --user`. Once RigCtl is installed and the service is enabled for the user, the daemon should start automatically with the user's login and remain available for the duration of the user session. The user should not need to start it manually before using `rigctl`.

Planned startup lifecycle:

```text
User logs in
    │
    ▼
systemd --user
    │
    ▼
rigctld.service
    │
    ├── initialize RigCtl
    ├── load configuration
    ├── restore persistent state where appropriate
    ├── initialize available subsystems
    └── create the RigCtl IPC socket
```

Expected responsibilities:

- hardware communication;
- persistent runtime state;
- device discovery and hotplug handling where useful;
- audio profile application;
- Crusher ANC 2 noise-control state and commands;
- owning the internal IPC socket;
- servicing CLI requests;
- later servicing Quickshell requests;
- maintaining state that should survive individual CLI invocations.

The daemon should remain small and mostly idle when no continuous work is required.

`rigctld` is not a system-wide root daemon in the initial implementation. Future fan control must not change that initial service scope.

---

## Logging, Diagnostics, and Shared Errors

`rigctld` will use structured Rust tracing integrated with the systemd user journal. The journal is the canonical daemon log destination; RigCtl will not maintain its own log files or rotation system.

The standard levels are `ERROR`, `WARN`, `INFO`, `DEBUG`, and `TRACE`. INFO should remain readable and focus on lifecycle and important state changes. Detailed implementation diagnostics belong at DEBUG, while raw HID, Bluetooth, and transport traces belong at TRACE and must be explicitly enabled.

Logs should prefer structured fields. Daemon filtering will use standard `RUST_LOG` / tracing environment filters so targeted diagnostics can be enabled without recompiling.

The CLI remains quiet during normal operation. Results go to stdout; errors, warnings, and diagnostics go to stderr. Increased verbosity must not alter or contaminate machine-readable stdout.

RigCtl uses one shared error model across the CLI, daemon, and every subsystem. All IPC errors require a stable machine-readable `code` and concise human-readable `message`, with an optional `details` object for structured context.

Subsystem and backend failures must be mapped into the shared vocabulary before crossing the IPC boundary. Raw Rust errors, D-Bus exceptions, HID errors, and other backend-specific details belong only in verbose/debug diagnostics.

Logging a failure never replaces returning a structured error to the caller. Partial completion remains failure; rollback status may be included in error details, and RigCtl must not claim transactionality that the underlying hardware does not provide.

Logs must exclude credentials, pairing keys/material, private keys, unrelated captured application data, and personal information. Hardware identifiers should be logged only when diagnostically useful and generally at DEBUG or TRACE.

---

## Subsystem Terminology

Mouse, keyboard, audio, and the later fan controller are **subsystems**.

RigCtl will not implement:

- downloadable plugins;
- independently managed optional modules;
- a module marketplace;
- a custom dependency manager;
- runtime installation/removal of subsystems.

The operating system package manager remains responsible for installing and removing RigCtl.

Hardware-specific code may still be separated internally when there is a concrete engineering or reuse benefit. The Ajazz protocol implementation is the strongest example because a complete Linux implementation could be valuable to the wider community.

---

## Mouse Controller

The mouse subsystem targets the **Razer Viper V3 HyperSpeed** by communicating directly with `openrazer-daemon` through D-Bus, without requiring Polychromatic or another third-party OpenRazer CLI frontend at runtime.

Planned functionality:

- device detection;
- DPI configuration;
- polling-rate configuration;
- named RigCtl mouse profiles;
- DPI stages where verified;
- battery information;
- additional supported device settings;
- direct communication with the OpenRazer backend.

Example commands:

```bash
rigctl mouse status
rigctl mouse dpi get
rigctl mouse dpi set 800
rigctl mouse polling-rate get
rigctl mouse polling-rate set 1000

rigctl mouse profile list
rigctl mouse profile current
rigctl mouse profile apply gaming
rigctl mouse profile create gaming
rigctl mouse profile delete gaming
```

The OpenRazer-specific implementation should remain independent from CLI formatting and future Quickshell UI code. Third-party OpenRazer CLI tools may still be used manually for development, debugging, and hardware capability testing.

### Mouse Profiles

RigCtl will support multiple named mouse profiles stored as separate configuration files under:

```text
$XDG_CONFIG_HOME/rigctl/profiles/mouse/
```

These are Linux-side RigCtl profiles. The Viper V3 HyperSpeed has one onboard hardware profile/configuration, so RigCtl must never present it as having multiple onboard profile slots.

Applying a named RigCtl profile writes its supported settings through `rigctld` and OpenRazer to the mouse's one active hardware configuration. The exact persistence of each OpenRazer setting to onboard memory must be verified experimentally rather than assumed.

Initial profile candidates include DPI, polling rate, DPI stages, active DPI stage, and lift-off distance. Additional fields may be added only after they can be read reliably, validated safely, written reliably, and read back after writing. The exact DPI-stage count, values, X/Y behavior, and OpenRazer representation remain to be verified on the physical device.

The distinction between creating a profile manually and a possible future `profile save` command for capturing current settings remains undecided.

### Mouse Hardware Safety

Hardware-changing commands and profile application must use verified capability information for the detected device and receiver. RigCtl must not infer capabilities from unrelated Razer models or provide a `--force` option that bypasses validation in the initial release.

The daemon must validate an entire mouse profile before the first hardware write. If any field is invalid or unsupported, it rejects the complete profile and performs zero writes.

Where supported, changes follow this sequence:

```text
Validate requested state
        │
        ▼
Read current settings
        │
        ▼
Write requested settings
        │
        ▼
Read settings back
        │
        ▼
Verify requested state
```

RigCtl reports success only after required values have been verified. If a profile write fails partway through, the daemon should attempt a safe best-effort restoration of captured settings and report whether rollback was incomplete. Because the hardware/backend may not support transactions, profile application must not be described as inherently atomic.

The daemon should avoid unnecessary writes when the requested value is already active. The normal CLI will not expose arbitrary OpenRazer commands, D-Bus payloads, USB messages, or unchecked DPI/polling-rate writes.

RigCtl will track the last successfully applied named mouse profile as persistent state, but it must distinguish that record from current hardware settings that may have changed outside RigCtl. Status should query hardware when practical and display only available, verified values.

---

## Keyboard Controller

The keyboard subsystem targets the **Ajazz AK820 MAX**.

Because its configuration protocol is undocumented, a significant part of the project will involve reverse engineering.

Possible investigation methods:

- USB/HID traffic capture;
- comparison of commands generated by the official software;
- HID report analysis;
- protocol documentation;
- binary analysis of the official application if necessary;
- controlled experiments against the physical keyboard.

Planned functionality:

- device detection;
- reading current configuration where possible;
- key remapping;
- Fn-layer configuration;
- macros;
- polling rate;
- keyboard profiles;
- other device-specific settings discovered during reverse engineering.

Example commands:

```bash
rigctl keyboard status
rigctl keyboard keymap get
rigctl keyboard remap capslock escape
rigctl keyboard profile apply gaming
```

The protocol implementation should not depend on the CLI, IPC representation, or future Quickshell UI.

```text
       Ajazz Protocol
            │
            ▼
      Keyboard Backend
            │
            ▼
          rigctld
            │
            ├── rigctl CLI
            └── future Quickshell UI
```

The protocol implementation and documentation should be designed so that they can potentially be reused or contributed to another Linux project later.

---

## Later — Fan Controller

Fan control will not be part of the initial RigCtl version. It is deferred until the mouse, keyboard, audio, daemon, IPC, and configuration foundations are stable.

The main reason is architectural and permission-related: reliable writes through Linux `hwmon` may require additional permission handling. Fan control must not complicate the initial privilege model or force the entire user-level `rigctld` daemon to run as root.

The fan subsystem focuses on fan profiles and custom fan curves.

Planned future capabilities include:

- hardware monitoring through Linux `hwmon` / sysfs;
- fan discovery;
- temperature sensor discovery;
- named fan profiles;
- custom temperature-to-fan-speed curves;
- curve validation;
- profile persistence;
- continuous curve evaluation;
- safe fallback behavior;
- multiple fan and sensor handling;
- future Quickshell fan controls.

Example curve:

```text
30°C → 20%
45°C → 30%
60°C → 50%
75°C → 75%
85°C → 100%
```

CLI representation:

```bash
rigctl fans profile create silent \
  --curve "30:20,45:30,60:50,75:75,85:100"
```

The format is:

```text
temperature:fan-speed
```

Profiles can be applied with commands such as:

```bash
rigctl fans profile apply silent
rigctl fans profile apply balanced
rigctl fans profile apply performance
```

These commands are not required for the initial release.

At minimum, validation must ensure:

- temperatures are ordered correctly;
- fan speeds remain within valid limits;
- invalid or incomplete curves are not applied.

When implemented, custom curves will require continuous temperature evaluation through `rigctld` or a narrowly scoped helper selected by the future privilege design.

Still to define:

- temperature source selection;
- update interval;
- interpolation;
- hysteresis or smoothing;
- startup behavior;
- hardware failure behavior;
- safe fallback behavior;
- multiple fan/sensor handling.

Before implementation, the project must inspect the actual target motherboard and `hwmon` interface, then select the narrowest appropriate permission mechanism. Possible approaches include narrowly scoped udev or sysfs permissions, a dedicated privileged helper, or another minimal privilege-separation mechanism.

No privileged fan-control architecture has been selected yet.

---

## Audio

The Audio subsystem is part of the **initial RigCtl version** and has two separate responsibilities:

```text
Audio
│
├── Linux EQ
│   ├── PipeWire
│   ├── parametric EQ
│   └── RigCtl EQ profiles
│
└── Crusher ANC 2 control
    └── headset noise-control mode
        ├── ANC
        ├── Off
        └── Stay-Aware
```

These responsibilities must remain separate internally. PipeWire performs EQ processing on the Linux host. The Crusher ANC 2 performs ANC and Stay-Aware processing on the headset; RigCtl only sends control commands and reads or tracks the resulting mode where technically possible.

PipeWire must not be used to imitate ANC or Stay-Aware functionality.

### EQ Profiles

The EQ responsibility focuses on named parametric profiles rather than trying to replace the entire desktop audio stack.

Initial EQ scope:

- select the exact PipeWire integration mechanism;
- define EQ profiles;
- list profiles;
- determine the active profile;
- apply profiles;
- persist default-profile references and successful active-profile state separately;
- create and delete profiles;
- expose the same functionality to the CLI and future Quickshell UI.

Example commands:

```bash
rigctl audio status
rigctl audio profile list
rigctl audio profile current
rigctl audio profile apply neutral
rigctl audio profile apply music
rigctl audio profile apply gaming
rigctl audio profile create <name>
rigctl audio profile delete <name>
```

Possible profile names include:

```text
neutral
music
gaming
voice
```

PipeWire is the selected host EQ layer and profiles use a parametric-EQ model. The exact PipeWire integration mechanism, stable filter fields, and validation limits still need to be researched and finalized.

### Crusher ANC 2 Noise Control

Noise control is represented as one mutually exclusive state with exactly three valid modes:

```text
ANC
Off
Stay-Aware
```

It must not be modeled as independent boolean flags that could produce invalid combinations.

Planned initial commands:

```bash
rigctl audio noise-control get
rigctl audio noise-control set anc
rigctl audio noise-control set off
rigctl audio noise-control set stay-aware
```

The CLI contract uses explicit `get` and `set` operations for this setting, and the three-state model is fixed.

The exact Linux-side headset protocol is not yet known. Investigation must determine the communication channel, how the current mode is queried, how modes are selected, whether notifications are available, whether commands persist across reconnects or power cycles, and how concurrent control by another application behaves. Reverse engineering of the Skullcandy mobile application's headset communication may be required.

The initial implementation requires device detection, selection of all three modes, exposure of the current known mode, and CLI/daemon integration. Adjustable ANC and Stay-Aware intensity are later enhancements; their ranges and CLI syntax must not be defined until the protocol has been mapped.

Noise-control mode and EQ profile are separate settings. Changing an EQ profile must not automatically change the headset mode in the initial design. A combined preset system may be considered later only if a concrete use case appears.

---

## Configuration

RigCtl will use human-readable TOML configuration and follow the XDG Base Directory conventions.

The primary configuration file is:

```text
$XDG_CONFIG_HOME/rigctl/config.toml
```

with `~/.config/rigctl/config.toml` as the default when `XDG_CONFIG_HOME` is not set.

The configuration model separates global configuration, named profiles, and persistent runtime state. `config.toml` contains only global settings and default-profile references; subsystem profile contents are not embedded in it. Mouse, audio EQ, and keyboard profiles are separate TOML files:

```text
$XDG_CONFIG_HOME/rigctl/profiles/mouse/<profile>.toml
$XDG_CONFIG_HOME/rigctl/profiles/audio/<profile>.toml
$XDG_CONFIG_HOME/rigctl/profiles/keyboard/<profile>.toml
```

Fan profiles may follow the same convention when fan control is implemented later.

Every `config.toml` and profile document requires `schema_version = 1`. Configuration schema versions are independent from the RigCtl application version.

Profile identity comes from the filename rather than a duplicate `name` field. Names use lowercase kebab-case with only `a-z`, `0-9`, and `-`, beginning and ending with an alphanumeric character. Names must be unique within a subsystem, while different subsystems may reuse the same name.

`rigctld` is the authoritative reader and writer for configuration and profiles. The CLI and future Quickshell UI perform configuration operations through IPC and must not implement separate parsers or file mutation. Reading or using in-memory defaults must never rewrite a file. Intentional writes should use a replacement strategy that preserves the previous valid file if a write fails.

Unknown fields are rejected in the initial schemas. Missing required fields produce `invalid_profile`; optional fields use defaults explicitly defined by the schema. A complete profile is parsed and validated—including schema, structure, semantics, and hardware capability—before any hardware write begins.

The configured default profile and the last successfully applied active profile are different concepts. Defaults belong in `config.toml`; successful active-profile state may be stored under `$XDG_STATE_HOME/rigctl/` and must not be maintained by rewriting `config.toml`. Persisted state is advisory, while actual hardware/backend state remains authoritative where it can be queried.

A missing referenced default profile must not be created automatically or replaced silently with an unrelated profile. The daemon reports and logs the configuration problem; safe subsystem-specific fallback behavior remains possible where explicitly defined.

Only schema version `1` is required initially. Future migration must be deliberate, versioned, daemon-owned, validated after conversion, and recoverable if files are rewritten. Unknown or unsupported schema versions produce a clear error.

Persistent daemon state that should survive restarts without being maintained manually as configuration will use `$XDG_STATE_HOME/rigctl/`. Transient files, including the IPC socket, will use `$XDG_RUNTIME_DIR/rigctl/`.

RigCtl should create `$XDG_CACHE_HOME/rigctl/` only if a real caching need appears. It does not currently require `$XDG_DATA_HOME/rigctl/`.

Still to define:

- override precedence beyond the documented default-profile references;
- exact hardware-dependent mouse fields after verification;
- exact keyboard schema after Ajazz protocol research;
- exact audio filter field names and validation limits after the PipeWire prototype;
- persistent state schema;
- subsystem-specific safe fallback behavior for a missing default profile.

The shared schema contract is finalized for implementation; intentionally deferred subsystem fields must be documented when verified.

---

## IPC

RigCtl's internal IPC will use a Unix domain stream socket owned by `rigctld`.

The `rigctl` CLI and future Quickshell UI will connect as clients. Messages will use UTF-8 newline-delimited JSON.

The initial IPC protocol version is `1` and is versioned independently from the RigCtl application. Every message includes a protocol `version` and one of three message types:

- `request`;
- `response`;
- `event`.

Requests and responses are correlated by ID, and each response contains either a successful result or one shared structured error.

Persistent clients may send multiple requests over one connection and explicitly subscribe to selected daemon-originated events. Responses are not guaranteed to arrive in request order. The future Quickshell UI will use this same protocol rather than a separate UI backend.

The protocol enforces a bounded message size. Malformed or oversized messages close only the offending client connection; `rigctld` remains running and performs no operation from a partial request.

Unix user permissions are the v1 access-control boundary, so no additional authentication token is required.

D-Bus will not be used for RigCtl's own internal IPC. It remains an external backend transport where required, including communication with `openrazer-daemon`.

Systemd socket activation is not required for the initial version because `rigctld` is expected to remain active as a lightweight user-level daemon.

Because `$XDG_RUNTIME_DIR` is session-scoped, the IPC socket will be recreated whenever the daemon starts.

---

## Future Quickshell Interface

A graphical interface is planned only after the CLI/daemon backend is mature.

It is expected to be implemented through **Omarchy / Quickshell**.

Potential UI capabilities:

### Mouse

- DPI;
- polling rate;
- battery information.
- named RigCtl profile selection and management;
- current verified DPI stage where supported.

### Keyboard

- visual keymap;
- remapping;
- Fn-layer editing;
- macros;
- profiles.

### Fans

- active profile after the fan subsystem is implemented;
- profile management after the fan subsystem is implemented;
- temperatures and fan state after the fan subsystem is implemented;
- graphical fan-curve editing after the fan subsystem is implemented.

### Audio

- active EQ profile;
- profile switching;
- graphical EQ editing if useful.
- Crusher ANC 2 noise control as a three-way exclusive selector for ANC, Off, and Stay-Aware.

The UI must remain a thin client and must not contain hardware-specific protocol logic.

---

## Resource Usage

Low resource usage is a core goal.

Principles:

- one lightweight `rigctld` daemon;
- automatic startup with the user session;
- availability for the duration of the user session;
- no Electron runtime;
- no browser frontend;
- no local HTTP server solely for UI purposes;
- no separate daemon for each subsystem;
- no custom plugin manager;
- no dedicated RigCtl log directory or log-rotation process;
- minimize polling;
- prefer event-driven behavior where practical;
- perform continuous work only where technically required.

The initial subsystems should avoid unnecessary polling. Fan-curve evaluation will require periodic work only after the fan subsystem is implemented.

---

## Linux Integration

RigCtl primarily targets **Omarchy / Arch Linux**.

Expected integration points:

- systemd;
- systemd journal;
- udev;
- D-Bus where external services require it;
- USB/HID;
- hwmon / sysfs for later fan control;
- the Linux audio stack;
- XDG filesystem conventions.

### systemd

`systemd` will manage `rigctld` as `rigctld.service` under `systemd --user` for the initial mouse, keyboard, and audio subsystems.

After RigCtl has been installed and configured for the user, the service should start automatically on future user logins. Packaging will include the service, but the exact package-time enablement mechanism remains undecided.

Systemd socket activation is not required for the initial version.

The exact unit contents, restart policy, dependency ordering, failure backoff, shutdown timeout, default log filter, service environment settings, and enablement instructions will be defined during daemon and packaging implementation.

### udev

`udev` rules may be required for:

- HID/USB permissions;
- device access;
- hotplug handling.

This is especially relevant to the Ajazz keyboard.

The permissions required by the initial subsystems still need to be finalized. Future fan control must not force the entire daemon to run as root.

---

## Packaging

Arch Linux packaging is the initial target.

Conceptually, the project should install as one package:

```text
rigctl
```

It may contain:

- `rigctl`;
- `rigctld`;
- `rigctld.service`;
- udev rules;
- default configuration;
- documentation;
- other required integration files.

RigCtl will not implement its own package manager.

The required packaging end state is that, after installation and user configuration, `rigctld` starts automatically on future logins. Exact `systemctl --user` instructions or package-time enablement behavior remain to be defined.

---

## Engineering Challenges

### 1. Ajazz AK820 MAX Reverse Engineering

Expected workflow:

```text
   Official software
          │
          ▼
 USB/HID traffic capture
          │
          ▼
   Packet comparison
          │
          ▼
  Protocol hypothesis
          │
          ▼
 Controlled experiment
          │
          ▼
 Linux implementation
```

The goal is both working Linux control and useful protocol documentation for the community.

### 2. Daemon and IPC Implementation

The CLI and future Quickshell UI share one versioned Unix-socket + JSONL backend contract. The main implementation challenge is keeping request dispatch, concurrent clients, structured errors, event subscriptions, and daemon lifecycle behavior reliable without unnecessary complexity.

### 3. Different Linux Hardware Interfaces

```text
Mouse       → openrazer-daemon / D-Bus
Keyboard    → USB / HID
Audio EQ    → PipeWire / parametric EQ
Crusher     → headset control protocol (undecided)
Fans        → hwmon / sysfs (later)
```

These backends need a consistent user-facing experience without forcing an artificial plugin architecture.

### 4. Privileges and Permissions

Keyboard HID access may require explicit permissions. Later fan control may require privileged writes or a narrowly scoped helper. RigCtl should use the minimum privileges required and must not assume the entire daemon runs as root.

### 5. Later Fan Safety

When fan control is implemented, curve validation, startup behavior, failures, and safe fallback behavior must be documented and tested carefully.

### 6. Audio EQ Integration and Schema

The project must finalize the exact PipeWire EQ integration mechanism and the stable parametric-EQ schema details, including supported filter types and validation limits, without turning RigCtl into a general-purpose audio suite.

### 7. Crusher ANC 2 Protocol Investigation

The project must determine the verified Linux-side control mechanism for selecting and querying ANC, Off, and Stay-Aware without assuming a Bluetooth transport in advance.

### 8. Mouse Capability and Profile Safety

Mouse writes must be validated against the detected device and receiver, verified through read-back where supported, and protected against partially applying an invalid named profile.

### 9. Low Resource Usage

The daemon must provide persistent functionality while remaining lightweight and mostly idle.

---

## Rust Workspace Structure

RigCtl will use a virtual Cargo workspace with three first-party crates. This crate layout is finalized for the initial implementation:

```text
crates/
├── rigctl/       # CLI client
├── rigctld/      # daemon and subsystem implementations
└── rigctl-ipc/   # shared IPC data contracts
```

`rigctl` owns command parsing, IPC client behavior, output formatting, and exit behavior. It must not contain hardware or backend logic. `rigctld` owns configuration, state, logging, IPC serving, subsystem behavior, hardware validation, and error mapping. `rigctl-ipc` is the only first-party crate shared directly by both executables and contains wire-level data contracts without owning socket transport or subsystem behavior.

Mouse, keyboard, and audio begin as modules inside `rigctld`, not as separate crates. `rigctld` will contain a thin `main.rs` plus a testable library. Configuration and profile storage are authoritative in the daemon; the CLI accesses them through IPC rather than interpreting the files independently.

The repository will also contain documentation, packaging integration, and test fixtures. Exact source filenames may evolve without changing the three initial crate boundaries.

Ajazz and Crusher ANC 2 protocol research may remain in separate repositories, with reusable Rust implementations later becoming daemon dependencies. New first-party crates require a concrete reuse, dependency-isolation, testing, or protocol-ownership reason.

The Ajazz protocol is the clearest candidate for later extraction into a reusable library, but no additional first-party crate is part of the initial workspace.

---

## Development Plan

### Phase 1 — Foundation

- repository setup;
- Rust workspace;
- `rigctl` CLI skeleton;
- `rigctld` daemon skeleton;
- shared `rigctl-ipc` data-contract crate;
- versioned Unix socket + JSONL IPC;
- IPC protocol v1 request, response, error, and event types;
- configuration system;
- configuration and profile schema version 1;
- profile naming, validation, and safe file-write behavior;
- XDG configuration, profile, state, and runtime layout;
- automatic user-session daemon lifecycle;
- structured `tracing` / journald diagnostics;
- shared structured error model;
- systemd user-service integration;
- permissions required by the initial subsystems.

### Phase 2 — Mouse

- OpenRazer communication through D-Bus;
- device detection;
- device and receiver capability profiles;
- DPI control;
- polling rate;
- named mouse profile storage and management;
- DPI-stage support where verified;
- full-profile validation before hardware writes;
- write/read-back verification;
- best-effort restoration after partial profile failure where safe;
- battery information;
- CLI integration;
- daemon integration.

### Phase 3 — Audio

- select the PipeWire EQ integration mechanism;
- define EQ profile representation;
- profile listing;
- active profile detection;
- profile application;
- profile persistence;
- profile creation/deletion;
- detect the Skullcandy Crusher ANC 2;
- investigate and document its noise-control protocol;
- implement the exclusive ANC, Off, and Stay-Aware state model;
- expose the current known noise-control mode;
- implement noise-control CLI commands;
- CLI integration;
- daemon integration.

### Phase 4 — Keyboard

- USB/HID investigation;
- official software traffic capture;
- protocol documentation;
- device discovery;
- reading configuration where possible;
- key remapping;
- Fn layer;
- macros;
- polling rate;
- profiles;
- additional supported settings.

### Phase 5 — Initial Integration and Polish

- integration tests;
- mocked backend tests where possible;
- hardware-in-the-loop tests;
- Arch packaging;
- systemd user-service packaging;
- installation documentation;
- troubleshooting documentation;
- daemon hardening;
- configuration migration strategy;
- stable interface for future Quickshell integration.

### Later — Fan Control

- inspect the target `hwmon` implementation;
- determine minimum required privileges;
- choose a privilege-separation strategy;
- sensor discovery;
- fan discovery;
- fan profile format;
- fan curve format;
- curve validation;
- profile storage;
- continuous curve evaluation;
- safe fallback behavior;
- CLI integration;
- daemon integration;
- Quickshell fan controls.

### Later — Quickshell UI

After the CLI/daemon backend is mature:

- Omarchy / Quickshell integration;
- mouse controls;
- keyboard controls;
- fan controls and graphical curves after the fan subsystem has been implemented;
- audio EQ profile controls;
- Crusher ANC 2 noise control as a three-way exclusive selector;
- live hardware state where useful.

---

## Expected Initial Result

The initial RigCtl version should provide:

- Razer Viper V3 HyperSpeed configuration through RigCtl;
- named RigCtl mouse profiles layered over the device's single onboard hardware configuration;
- important Ajazz AK820 MAX configuration from Linux where reverse engineering permits;
- documented Ajazz protocol findings with potential community reuse;
- configurable and persistent audio EQ profiles;
- Crusher ANC 2 switching between ANC, Off, and Stay-Aware;
- CLI access to mouse, keyboard, and audio functionality;
- one lightweight, user-level `rigctld` backend;
- automatic `rigctld` startup with the user session;
- versioned Unix socket + JSONL IPC protocol v1;
- schema-versioned TOML global configuration and separate mouse, audio, and keyboard profile files;
- no browser or Electron frontend;
- architecture ready for later Quickshell integration.

Fan control remains an intended future RigCtl feature, but its absence must not block the initial release.

The project will be developed in a public GitHub repository with regular, traceable commits.
