# RigCtl — Technical Stack

## Overview

RigCtl is a lightweight Linux hardware and system-control project focused on:

- minimal resource usage;
- direct hardware access;
- CLI-first development;
- one lightweight daemon;
- Linux-native integration;
- reliable control of a fixed set of target subsystems;
- reusable device-specific protocol work where that provides real community value.

The initial version includes three subsystems:

- mouse;
- keyboard;
- PipeWire audio equalization and Skullcandy Crusher ANC 2 noise control.

Fan control remains part of the longer-term roadmap but is explicitly deferred until the initial daemon, IPC, configuration, mouse, keyboard, and audio foundations are stable.

RigCtl is **not** a generic plugin/module framework.

The application will primarily be implemented in **Rust**.

A browser frontend is no longer part of the architecture. A graphical interface may be added later through Omarchy / Quickshell and should consume the same backend as the CLI.

---

## Runtime Architecture

RigCtl will use two primary executables:

```text
rigctl    → command-line client
rigctld   → background daemon
```

High-level architecture:

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

Backend connections:

```text
Mouse       → OpenRazer / D-Bus
Keyboard    → USB / HID
Audio EQ    → PipeWire / parametric EQ
Crusher     → headset control protocol (undecided)
Fans        → hwmon / sysfs (later implementation)
```

RigCtl's internal IPC uses a Unix domain stream socket with UTF-8 newline-delimited JSON messages. `rigctld` owns the socket; `rigctl` and the future Quickshell UI are clients.

Both executables depend on the shared `rigctl-ipc` data-contract crate, while socket transport remains owned by the client and daemon themselves.

---

## Language

### Rust

Rust will be used for:

- the CLI;
- the daemon;
- hardware communication;
- subsystem implementations;
- configuration handling;
- IPC;
- profile handling;
- Linux integration where practical;
- tests.

Reasons for Rust include:

- memory safety without garbage collection;
- low runtime overhead;
- native binaries;
- strong Linux systems-programming support;
- strong type safety;
- mature CLI libraries;
- good async/IPC ecosystem;
- ability to isolate reusable libraries where justified.

---

## CLI

### clap

The CLI will use the `clap` crate.

General form:

```text
rigctl [GLOBAL OPTIONS] <subsystem> <resource/action> ...
```

The CLI will prefer explicit verbs such as `get`, `set`, `list`, `current`, `apply`, `create`, and `delete` rather than overloading positional values.

Preferred:

```bash
rigctl mouse dpi set 800
```

rather than:

```bash
rigctl mouse dpi 800
```

Initial command hierarchy:

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
rigctl mouse profile create gaming
rigctl mouse profile delete gaming

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
rigctl audio profile create music
rigctl audio profile delete music
rigctl audio noise-control get
rigctl audio noise-control set anc
rigctl audio noise-control set off
rigctl audio noise-control set stay-aware
```

Additional commands may be added, but they should follow the same explicit hierarchy.

### Output contract

Normal output is concise and human-readable. Commands that return data support `--json`.

Examples:

```bash
$ rigctl mouse dpi get
800 DPI
```

```bash
$ rigctl audio noise-control get
ANC
```

Successful JSON output represents the requested object directly and is not wrapped in a redundant `{ "success": true, "data": ... }` envelope. The process exit code communicates success or failure.

Example:

```bash
rigctl audio status --json
```

```json
{"device":"Skullcandy Crusher ANC 2","eq_profile":"gaming","noise_control":"anc"}
```

RigCtl maintains strict stream separation:

```text
stdout → requested command result
stderr → errors, warnings, and diagnostics
```

Debug and diagnostic logging must never contaminate normal stdout output.

### Global options

Initial global options:

```text
--help
--version
--json
--verbose
```

Short-form `-v` / `-vv` verbosity may be added later if useful. A custom socket path is not part of the normal public CLI; a development-only option or environment variable may be added later if testing requires it.

### Exit codes

```text
0   Success
1   Runtime or operation failure
2   Invalid CLI usage
3   rigctld unavailable
```

Detailed failures use stable structured error identifiers rather than a separate process exit code for every condition.

### Validation responsibilities

`rigctl` validates:

- CLI syntax;
- argument presence;
- primitive types;
- universally invalid values.

`rigctld` performs authoritative validation for:

- actual device capabilities;
- supported value ranges;
- supported discrete values;
- current connection type;
- backend availability;
- current hardware state.

For example, `rigctl mouse dpi set banana` is rejected before IPC. A syntactically valid request such as `rigctl mouse dpi set 30000` still requires daemon-side capability validation before any write.

Normal execution model:

```text
User
  │
  ▼
rigctl
  │
  │ IPC request
  ▼
rigctld
  │
  ▼
Subsystem backend
```

### Daemon availability

`rigctl` must not silently launch `rigctld`; systemd owns the daemon lifecycle. If the socket is unavailable, the CLI emits a `daemon_unavailable` error and exits with code 3.

```text
systemd → owns rigctld lifecycle
rigctl  → communicates with rigctld
```

---

## Daemon

### `rigctld`

`rigctld` will be a lightweight long-running process.

It will run as `rigctld.service` under `systemd --user`. Once installed and enabled for the user, it should start automatically with the user's login and remain available for the duration of the user session.

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

- servicing CLI requests;
- hardware communication;
- device discovery;
- hotplug handling where useful;
- persistent runtime state;
- audio profile application;
- Crusher ANC 2 noise-control state and commands;
- ownership of the internal IPC socket;
- later servicing Quickshell requests;
- exposing state changes when needed.

The daemon should avoid unnecessary polling and remain mostly idle when possible.

A single daemon is preferred over separate background services per subsystem.

The initial daemon is a user-level service, not a system-wide root daemon. The CLI should not need to start it manually before sending requests.

---

## IPC

RigCtl will use Inter-Process Communication (IPC) over a Unix domain stream socket between `rigctld`, the `rigctl` CLI, and the future Quickshell UI.

### Transport, framing, and access control

```text
Transport:       Unix domain stream socket
Location:        $XDG_RUNTIME_DIR/rigctl/rigctld.sock
Serialization:   JSON
Framing:         one JSON object per line
Encoding:        UTF-8
Message limit:   1 MiB
```

`rigctld` owns the socket. The `rigctl` CLI and future Quickshell UI connect as clients.

Planned filesystem permissions are:

```text
$XDG_RUNTIME_DIR/rigctl/                 0700
$XDG_RUNTIME_DIR/rigctl/rigctld.sock     0600
```

Unix filesystem permissions are the v1 access-control boundary. No additional authentication token is required for protocol v1.

```text
                  Quickshell
                      │
                      │ UTF-8 JSONL
                      ▼
rigctl ── UTF-8 JSONL ──► rigctld
                              │
               ┌──────────────┼──────────────┐
               ▼              ▼              ▼
             Mouse         Keyboard         Audio
```

### Protocol envelope and version

The protocol defines three message types:

```text
request
response
event
```

Every message contains `version` and `type`. The initial IPC protocol version is `1` and is independent from the RigCtl application version.

```rust
pub const PROTOCOL_VERSION: u32 = 1;
```

This allows, for example, RigCtl application version `0.4.2` to continue using IPC protocol version `1`.

### Requests

A request contains `version`, `type`, `id`, `method`, and `params`:

```json
{
  "version": 1,
  "type": "request",
  "id": 42,
  "method": "mouse.dpi.set",
  "params": {
    "dpi": 800
  }
}
```

Request IDs are `u64` values and need to be unique only among outstanding requests on the same connection. Every response includes the matching ID.

IPC methods use dot-separated `subsystem.resource.action` names with snake_case components. Representative v1 names include:

```text
system.status

mouse.status
mouse.dpi.get
mouse.dpi.set
mouse.polling_rate.get
mouse.polling_rate.set
mouse.profile.list
mouse.profile.current
mouse.profile.apply
mouse.profile.create
mouse.profile.delete

keyboard.status
keyboard.profile.list
keyboard.profile.current
keyboard.profile.apply
keyboard.keymap.get
keyboard.remap.set

audio.status
audio.profile.list
audio.profile.current
audio.profile.apply
audio.profile.create
audio.profile.delete
audio.noise_control.get
audio.noise_control.set

events.subscribe
```

The shell-facing CLI may use kebab-case, such as `polling-rate`, while the IPC name uses `polling_rate`.

`params` is always a JSON object. A request without arguments uses an empty object rather than omitting the field, using `null`, or using a positional array:

```json
{
  "version": 1,
  "type": "request",
  "id": 1,
  "method": "mouse.dpi.get",
  "params": {}
}
```

### Responses

A successful response contains `version`, `type`, `id`, `status`, and `result`:

```json
{
  "version": 1,
  "type": "response",
  "id": 42,
  "status": "ok",
  "result": {
    "dpi": 800
  }
}
```

`result` should normally be an object even when it contains one value, leaving room for compatible related fields later.

Error responses use the shared RigCtl error model:

```json
{
  "version": 1,
  "type": "response",
  "id": 42,
  "status": "error",
  "error": {
    "code": "unsupported_value",
    "message": "Unsupported polling rate: 700 Hz",
    "details": {
      "requested": 700,
      "supported": [125, 500, 1000],
      "unit": "Hz"
    }
  }
}
```

A response is exactly one of:

```text
status = ok
    └── result

status = error
    └── error
```

It must not contain both `result` and `error`.

### Events and subscriptions

`rigctld` may send daemon-originated events to persistent clients. Events have no request ID and use dot-separated names:

```json
{
  "version": 1,
  "type": "event",
  "event": "mouse.battery.changed",
  "data": {
    "percent": 72
  }
}
```

Initial event names may include:

```text
mouse.connected
mouse.disconnected
mouse.battery.changed
mouse.profile.changed

keyboard.connected
keyboard.disconnected
keyboard.profile.changed

audio.output.changed
audio.profile.changed
audio.noise_control.changed
```

Clients do not receive every event automatically. A persistent client subscribes to the exact event names it needs:

```json
{
  "version": 1,
  "type": "request",
  "id": 1,
  "method": "events.subscribe",
  "params": {
    "events": [
      "mouse.battery.changed",
      "audio.profile.changed",
      "audio.noise_control.changed"
    ]
  }
}
```

The response returns the accepted exact names in `result.subscribed`. Wildcard subscriptions are not required for v1.

### Connection model

The protocol supports multiple outstanding requests on one connection. Responses are correlated by request ID, and clients must not assume that responses arrive in request order. Events may arrive between responses.

The CLI normally opens a short-lived connection, sends one request, receives its response, and disconnects. The future Quickshell UI may keep one persistent connection for requests and subscribed events. Both use the same protocol.

### Protocol failures

If a request uses an unsupported protocol version but can still be parsed well enough to recover its ID, the daemon returns `protocol_mismatch` in a normal v1 response envelope and reports its supported version in `details`.

If input is valid JSON and follows enough of the envelope to respond, but has a missing required parameter, an unknown method, or an invalid field combination, the daemon returns `invalid_request`.

Malformed JSON without a reliably recoverable request ID does not introduce a second error format. The daemon logs the condition, closes only that client connection, and remains running.

A message exceeding 1 MiB is handled the same way: the daemon logs the condition, closes the offending connection, performs no hardware operation from the partial request, and remains running.

### Rust representation

The stable wire types are defined in the shared `rigctl-ipc` crate and map to Serde-tagged enums or equivalent typed structures for `Request`, `Response`, `Event`, `RigCtlError`, and `ProtocolVersion`. The crate defines the data contract but does not own the socket transport.

### Communication layers

```text
User
 │
 ▼
rigctl
 │
 │ Unix socket + JSONL
 ▼
rigctld
 │
 ├── D-Bus ─────► openrazer-daemon
 ├── HID/USB ───► Ajazz AK820 MAX
 ├── PipeWire ──► Audio EQ processing
 ├── TBD ───────► Crusher ANC 2 control
 └── hwmon ─────► Fan control (later)
```

D-Bus is not used for RigCtl's own internal IPC. OpenRazer D-Bus is an external backend integration.

Systemd socket activation is not required for the initial version because `rigctld` is expected to remain active as a lightweight user-level daemon.

Because `$XDG_RUNTIME_DIR` is scoped to the user session, the runtime directory and socket will be recreated whenever `rigctld` starts.

The protocol v1 wire format documented above is considered finalized enough for implementation.

---

## Configuration

### serde + TOML

Configuration structures will use `serde`.

TOML is the human-readable configuration and profile format. The model is split into three responsibilities:

```text
global configuration
named profiles
persistent runtime state
```

Global configuration and profiles are user-authored documents under the XDG configuration root. Runtime state that must survive daemon restarts but is not configuration belongs under the XDG state root.

### Filesystem layout

The primary configuration file is:

```text
$XDG_CONFIG_HOME/rigctl/config.toml
```

When `XDG_CONFIG_HOME` is not set, the default is:

```text
~/.config/rigctl/config.toml
```

Profiles use one TOML file per profile:

```text
$XDG_CONFIG_HOME/rigctl/
├── config.toml
└── profiles/
    ├── mouse/
    │   ├── default.toml
    │   ├── gaming.toml
    │   └── desktop.toml
    │
    ├── audio/
    │   ├── neutral.toml
    │   ├── music.toml
    │   └── gaming.toml
    │
    └── keyboard/
        ├── default.toml
        └── gaming.toml
```

Audio profiles use:

```text
$XDG_CONFIG_HOME/rigctl/profiles/audio/<profile>.toml
```

Mouse profiles use:

```text
$XDG_CONFIG_HOME/rigctl/profiles/mouse/<profile>.toml
```

Keyboard profiles use:

```text
$XDG_CONFIG_HOME/rigctl/profiles/keyboard/<profile>.toml
```

Fan profiles may follow the same convention when fan control is implemented later.

Persistent state uses:

```text
$XDG_STATE_HOME/rigctl/
```

with `~/.local/state/rigctl/` as the default. It may record the last successfully applied mouse, keyboard, or audio profile and other daemon state that should survive restarts. Active-profile state must not be stored by rewriting `config.toml` during normal operation.

### Schema versioning

Every RigCtl TOML document requires:

```toml
schema_version = 1
```

This applies to `config.toml` and every mouse, keyboard, and audio profile. The schema version is independent from the RigCtl application version and changes only for an incompatible format change requiring migration or special handling.

Only schema version `1` must be supported initially. An unknown or unsupported version produces a clear error rather than being parsed optimistically.

### Global configuration

`config.toml` contains global daemon or subsystem-wide options and references to default profiles. It must not contain full mouse profiles, keyboard layouts, EQ filter lists, active runtime state, or temporary device state.

Conceptual initial structure:

```toml
schema_version = 1

[mouse]
default_profile = "gaming"

[keyboard]
default_profile = "default"

[audio]
default_profile = "music"
```

Subsystem behavior is not duplicated into `config.toml` when it belongs in a profile.

### Profile identity and filenames

A profile's canonical name is its filename without `.toml`; the file does not contain a duplicate `name` field. Filenames use lowercase kebab-case with the conservative character set `a-z`, `0-9`, and `-`. The first and last characters are alphanumeric.

Valid examples:

```text
default
gaming
competitive-fps
music-neutral
profile2
```

Invalid examples:

```text
Gaming
my profile
../gaming
gaming/profile
-gaming
gaming-
```

Names are unique within a subsystem directory. Mouse, keyboard, and audio may each have a separate profile named `gaming` because they occupy different namespaces.

### Ownership and read behavior

`rigctld` is the authoritative reader and writer of `config.toml`, profile files, and schema migrations. The CLI and future Quickshell UI perform these operations over IPC and must not implement a second parser or profile-management path.

Reading configuration must not modify it. RigCtl does not rewrite a file merely because it was parsed, defaults were applied in memory, Rust field order differs, the application was upgraded, or a status command was run. A write occurs only after an intentional user operation that modifies configuration or profiles.

Exact preservation of comments, whitespace, and key ordering is not required initially. An intentional rewrite may normalize formatting, but avoiding unnecessary rewrites minimizes this impact. Comment-preserving editing may be considered later if a concrete need appears.

### Field handling and validation

Unknown fields are configuration errors in the initial schemas rather than being ignored. Diagnostics should identify the unknown field where practical so misspellings, stale fields, and incompatible manual edits do not disappear silently.

Missing required profile fields produce `invalid_profile`. Missing optional fields use documented schema-defined defaults; individual clients must not guess defaults dynamically.

Every profile is fully validated before hardware changes begin:

```text
Read TOML
   │
   ▼
Parse
   │
   ▼
Validate schema version
   │
   ▼
Validate field structure
   │
   ▼
Validate semantic constraints
   │
   ▼
Validate hardware capability
   │
   ▼
Only then apply
```

Any validation failure results in zero hardware writes. This applies to mouse, keyboard, audio, and future fan profiles.

### Profile application safety

Profile application follows:

```text
validate complete profile
        │
        ▼
capture current relevant state where possible
        │
        ▼
apply settings
        │
        ▼
read back / verify where supported
        │
        ▼
persist successful active-profile state
```

A partial application is an overall failure. The daemon attempts best-effort rollback where safe, does not claim unsupported transactional behavior, and does not update active-profile state unless application succeeds.

### Profile references and active state

Default-profile references omit the `.toml` suffix. For example, `audio.default_profile = "music"` resolves to `$XDG_CONFIG_HOME/rigctl/profiles/audio/music.toml`.

If a referenced default profile does not exist, the daemon does not create it, logs the configuration problem, and reports a clear error for the affected subsystem. A subsystem may use an explicitly documented safe fallback, but RigCtl must not silently select an unrelated profile.

The configured default and currently active profile are different concepts. The default belongs in `config.toml`; the last profile successfully applied by RigCtl may be stored under `$XDG_STATE_HOME/rigctl/`. That persisted value is advisory and does not prove the physical device still has those settings. Where practical, `rigctld` queries actual hardware or backend state because onboard controls, reconnects, firmware, other applications, or external tools may have changed it.

### Schema migration

Future migrations are deliberate and versioned, must preserve unknown user data, and validate the converted configuration. If automatic migration rewrites a file, it must provide a recoverable path or backup. Migration logic belongs to `rigctld`.

### Atomic file writes

Configuration and profile writes must not leave a truncated document. The preferred replacement strategy is:

```text
write temporary file
        │
        ▼
flush / close successfully
        │
        ▼
rename into final path
```

A failed write must leave the previous valid file intact. The exact Rust implementation may vary.

### Subsystem profile schemas

Mouse profiles are part of the initial implementation. Their base form is:

```toml
schema_version = 1

dpi = 800
polling_rate = 1000
```

Later verified fields may include an active DPI stage and DPI-stage definitions. The stable mouse schema exposes only settings verified as safely readable and writable through the actual Viper V3 HyperSpeed and OpenRazer path; exact hardware-dependent fields remain deferred until verification.

The keyboard profile schema remains intentionally deferred until the Ajazz AK820 MAX protocol is sufficiently understood. Likely categories include keymaps, the Fn layer, macros, polling rate, and device profile settings, but the schema must follow observed protocol capabilities rather than assumptions based on the Windows UI.

Audio profiles use a parametric-EQ model. The conceptual shape is:

```toml
schema_version = 1

preamp_db = -5.0

[[filter]]
type = "peaking"
frequency = 80.0
gain_db = 3.0
q = 0.8
```

The PipeWire prototype still needs to determine stable field names, supported filter types, frequency/gain/Q bounds, filter-count limits, channel behavior, and preamp constraints.

Crusher ANC 2 noise-control mode is not part of the EQ profile schema. ANC, Off, and Stay-Aware remain a separate mutually exclusive state, and applying an EQ profile must not change noise-control mode.

### Profile management and errors

Profile operations support `list`, `current`, `apply`, `create`, and `delete` where appropriate. `rigctld` remains the only component that mutates profile files.

Malformed TOML, an unsupported schema version, a missing required field, an unknown field, an invalid field combination or EQ filter, and an unsupported mouse capability all produce `invalid_profile`. Structured error details should identify the reason and relevant field or capability where practical.

The shared configuration/profile contract is finalized for implementation. Subsystem-specific fields that depend on hardware or PipeWire research remain intentionally deferred and must be documented when verified.

---

## Mouse Subsystem

### OpenRazer

The **Razer Viper V3 HyperSpeed** will be controlled by communicating directly with `openrazer-daemon` through D-Bus, without relying on Polychromatic or another third-party OpenRazer CLI frontend at runtime.

```text
    rigctld
       │
       │ D-Bus via zbus
       ▼
openrazer-daemon
       │
       ▼
 Razer Device
```

### zbus

The Rust `zbus` crate will be used for D-Bus communication with `openrazer-daemon`.

Planned capabilities:

- device detection;
- DPI configuration;
- polling-rate configuration;
- battery information;
- additional supported settings exposed by OpenRazer.

The backend should remain independent from CLI presentation and Quickshell code.

A separate mouse crate is not required merely for architectural symmetry.

RigCtl must not invoke a third-party OpenRazer CLI tool as part of its runtime. Such tools may still be used manually for development, debugging, and hardware capability testing.

OpenRazer's D-Bus interface is an external backend integration and is independent of RigCtl's internal Unix-socket IPC.

### RigCtl mouse profiles

Named mouse profiles are separate RigCtl-managed configuration files:

```text
$XDG_CONFIG_HOME/rigctl/profiles/mouse/
```

Default location:

```text
~/.config/rigctl/profiles/mouse/
```

The Viper V3 HyperSpeed has one onboard hardware profile/configuration. RigCtl can store multiple named Linux-side profiles, but it must never present the device as having multiple onboard profile slots.

```text
RigCtl named profile
        │
        ▼
     rigctld
        │
        ▼
     OpenRazer
        │
        ▼
Viper V3 HyperSpeed
(one active hardware configuration)
```

The persistence behavior of each setting written through OpenRazer must be verified experimentally. RigCtl must not assume that every successful write is stored in onboard memory.

Profile commands:

```bash
rigctl mouse profile list
rigctl mouse profile current
rigctl mouse profile apply gaming
rigctl mouse profile create gaming
rigctl mouse profile delete gaming
```

A future `rigctl mouse profile save gaming` convenience command may capture supported current settings. The distinction between `create` and `save` remains undecided and should not be duplicated prematurely.

Initial profile-field candidates include:

- DPI;
- polling rate;
- DPI stages;
- active DPI stage;
- lift-off distance.

Additional fields such as button configuration, sleep timeout, or battery-related settings may be added only after each value can be read reliably, validated safely, written reliably, and read back after writing. A field is not added merely because Synapse exposes it.

OpenRazer contains device-specific DPI-stage support for the Viper V3 HyperSpeed. Conceptual profile representation:

```toml
schema_version = 1

dpi = 800
polling_rate = 1000
active_dpi_stage = 2

[[dpi_stages]]
dpi = 400

[[dpi_stages]]
dpi = 800

[[dpi_stages]]
dpi = 1600
```

The shared profile contract and schema version are final, but this DPI-stage extension is not. The supported stage count and values, X/Y DPI behavior, and OpenRazer representation must be verified on the physical device.

### Profile validation and application

The daemon validates the complete profile before the first hardware write:

```text
Load profile
    │
    ▼
Validate schema
    │
    ▼
Validate every field against device capabilities
    │
    ▼
Only then begin hardware writes
```

If any field is unsupported or invalid, the daemon rejects the entire profile and performs zero writes.

Where supported, application uses:

```text
Validate full profile
        │
        ▼
Read current configuration
        │
        ▼
Apply profile settings
        │
        ▼
Read settings back
        │
        ▼
Verify requested state
```

RigCtl reports success only after all required settings have been verified. If a write fails partway through, the daemon reports failure and attempts best-effort restoration of captured settings where safe. The error must indicate if rollback was incomplete. Because OpenRazer and the hardware may not provide transactions, profile application is not inherently atomic.

The daemon tracks the last successfully applied named profile under `$XDG_STATE_HOME/rigctl/`, but must distinguish that record from current hardware settings, which may be changed outside RigCtl. Status should query hardware where practical and expose only available, verified values in both human-readable and JSON forms.

Conceptual human-readable status:

```text
Device:          Razer Viper V3 HyperSpeed
Profile:         gaming
DPI:             800
DPI stage:       2
Polling rate:    1000 Hz
```

### Capability validation

Before any configuration write, `rigctld` determines the connected device and receiver/connection type, then selects a capability profile. Capability sources, strongest first, are:

1. reliable capabilities exposed by the backend/device;
2. official hardware documentation;
3. values verified experimentally on the exact device/receiver combination.

Capabilities must not be inferred from unrelated Razer models. The public CLI has no `--force` option and exposes no arbitrary OpenRazer commands, D-Bus payloads, USB control messages, or unchecked DPI/polling-rate writes in the initial release.

#### DPI

The officially documented maximum for the Viper V3 HyperSpeed's Focus Pro 30K sensor is 30,000 DPI.

Existing Synapse testing reports 100–30,000 DPI in 100-DPI steps, but the minimum and step must be verified against the physical device and OpenRazer before becoming the RigCtl capability contract. A range-plus-step rule may be used only after verification.

Until that verification is complete, RigCtl must not silently assume values below the verified safe range.

#### Polling rate

With the stock HyperSpeed receiver, current OpenRazer-related testing has successfully written and read back these discrete values:

```text
125 Hz
500 Hz
1000 Hz
```

They are a discrete allowed set, not a numeric range; a value such as 700 Hz is invalid.

Razer documents up to 8,000 Hz with the separate HyperPolling Wireless Dongle. That receiver requires a separate capability profile. Rates such as 2,000, 4,000, and 8,000 Hz remain unavailable until the compatible receiver is detected and the exact receiver/backend combination is verified.

### Write verification

Hardware writes use write-then-read-back verification whenever supported. A mismatch produces the stable `verification_failed` error. Unsupported values produce `unsupported_value` before any write is attempted and may include verified supported values or ranges.

Where practical, the daemon reads the current setting before changing it. A request for the already-active value succeeds without an unnecessary hardware write.

---

## Keyboard Subsystem

The **Ajazz AK820 MAX** will communicate directly through USB/HID.

Because its configuration protocol is undocumented, this work includes both implementation and reverse engineering.

### hidapi

`hidapi` is the preferred initial library for:

- device discovery;
- HID reports;
- feature reports;
- reading HID data;
- writing HID data.

### rusb

If lower-level USB access becomes necessary, `rusb` may be used.

Conceptual layering:

```text
Ajazz protocol
     │
     ├── report structures
     ├── command encoding
     ├── response decoding
     ├── device discovery
     └── transport
             │
             ▼
        Keyboard backend
             │
             ▼
           rigctld
```

The Ajazz protocol implementation should not depend on:

- CLI formatting;
- IPC representation;
- Quickshell;
- mouse/fan/audio code.

This is the part of RigCtl with the clearest potential for independent community reuse. If it becomes sufficiently complete and cleanly separable, it may become its own Rust crate. That split is not mandatory yet.

---

## Later — Fan Subsystem

Fan control will not be part of the initial RigCtl version. Reliable writes through Linux `hwmon` may require privileges or permission handling that should not complicate the initial user-level daemon.

The future fan subsystem will target:

```text
/sys/class/hwmon/
```

Planned capabilities:

- sensor discovery;
- fan discovery;
- profiles;
- custom curves;
- profile persistence;
- curve validation;
- applying fan curves;
- continuous evaluation through `rigctld`.

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

Fan CLI commands are not required for the initial release.

Conceptual runtime loop:

```text
     rigctld
        │
        ▼
 Read temperature
        │
        ▼
Evaluate fan curve
        │
        ▼
Calculate target PWM
        │
        ▼
   Apply value
        │
        ▼
 Wait / next update
```

Still to define:

- update interval;
- sensor selection;
- fan selection;
- interpolation;
- hysteresis;
- smoothing;
- multiple fan/sensor handling;
- safe startup behavior;
- safe fallback behavior;
- write-failure recovery;
- disappearing-sensor behavior.

Fan safety requirements must be documented before this subsystem is considered production-ready.

### Privilege model

Fan control must not force the entire `rigctld` daemon to run as root.

Before implementation, the project will inspect the actual target motherboard and `hwmon` interface, then choose the narrowest appropriate permission mechanism. Possible approaches include:

- narrowly scoped udev or sysfs permissions;
- a dedicated privileged helper;
- another minimal privilege-separation mechanism.

No privileged fan-control architecture has been selected yet.

---

## Audio Subsystem

The Audio subsystem is part of the **initial release** and contains two distinct internal responsibilities:

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

The EQ and Crusher control implementations must remain separate. PipeWire performs EQ processing on the Linux host. ANC and Stay-Aware processing occurs on the Crusher ANC 2 itself; RigCtl sends control commands and reads or tracks the resulting headset mode where technically possible.

PipeWire must not be used to imitate ANC or Stay-Aware functionality.

### PipeWire EQ

Planned capabilities:

- named parametric EQ profiles;
- profile listing;
- determining the active profile;
- profile application;
- profile persistence;
- profile creation/deletion;
- later graphical editing through Quickshell.

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

The selected default profile is referenced from `config.toml`:

```toml
schema_version = 1

[audio]
default_profile = "music"
```

Each EQ profile is stored independently, for example:

```text
$XDG_CONFIG_HOME/rigctl/profiles/audio/music.toml
```

PipeWire is the selected host EQ layer and the profile model is parametric EQ. The exact PipeWire integration mechanism, stable field names, and validation limits are **not yet selected**.

Before implementation, determine:

- which PipeWire mechanism hosts the EQ;
- mapping the parametric profile schema to the selected PipeWire mechanism;
- how filters are applied;
- persistence across device changes;
- output-device identification;
- unavailable/failure states.

### Crusher ANC 2 Noise Control

The current mode must be represented as a single mutually exclusive state rather than independent boolean flags:

```rust
enum NoiseControlMode {
    Anc,
    Off,
    StayAware,
}
```

Exactly one mode is active at a time. This prevents invalid combinations such as ANC and Stay-Aware both being enabled.

Planned initial commands:

```bash
rigctl audio noise-control get
rigctl audio noise-control set anc
rigctl audio noise-control set off
rigctl audio noise-control set stay-aware
```

The CLI contract uses explicit `get` and `set` operations for this setting, and the three-state model is fixed.

Conceptual status output:

```text
Device:         Skullcandy Crusher ANC 2
Noise control:  ANC
```

Initial implementation scope:

- detect the Skullcandy Crusher ANC 2;
- switch to ANC;
- switch to Off;
- switch to Stay-Aware;
- expose the current known mode;
- integrate control into `rigctld`;
- expose control through the CLI;
- preserve the architecture for later Quickshell control.

The exact Linux-side communication protocol is **not yet known**. Investigation must determine:

- whether control uses Bluetooth LE / GATT, another Bluetooth channel, or another vendor-specific mechanism;
- how the current mode is queried, if possible;
- how each mode is selected;
- whether state-change notifications are available;
- how ANC and Stay-Aware intensity are represented;
- whether commands persist across reconnects and power cycles;
- whether another application can control the headset concurrently.

Reverse engineering of the Skullcandy mobile application's headset communication may be required. RigCtl must not assume a Bluetooth control mechanism until it has been verified.

Adjustable ANC and Stay-Aware intensity are later enhancements. Their ranges and CLI format must not be defined until the protocol has been mapped. Other Skullcandy-specific features remain out of scope unless explicitly added later.

Noise-control mode and EQ profile are independent settings. Changing an EQ profile must not change the headset mode in the initial design. A combined preset system may be considered later only if there is a concrete use case.

RigCtl is not intended to replace a full mixer, Bluetooth manager, or desktop audio settings application.

---

## Future Quickshell Integration

A standalone browser UI is no longer planned.

The eventual GUI should be implemented through **Omarchy / Quickshell** and use the same backend as the CLI.

```text
Quickshell UI
      │
      │ same backend / IPC
      ▼
   rigctld
      ▲
      │
   rigctl CLI
```

Potential UI features:

- mouse DPI, polling rate, battery, and named RigCtl profiles;
- visual keyboard mapping, Fn layer, macros, and profiles;
- fan profiles, temperatures, and graphical curves after the fan subsystem is implemented;
- audio EQ profile switching and graphical editing;
- Crusher ANC 2 noise control as a three-way exclusive selector for ANC, Off, and Stay-Aware.

The noise-control UI must use one exclusive selector rather than three independent toggles.

Quickshell integration is intentionally deferred until the CLI/daemon interface is mature.

---

## Linux Integration

### systemd

`systemd` will manage `rigctld` as `rigctld.service` under `systemd --user` for the initial mouse, keyboard, and audio subsystems.

Once RigCtl has been installed and the service enabled for the user, `rigctld` should start automatically with the user's login and remain available throughout the user session.

Systemd socket activation is not required for the initial version.

Future fan-control privilege requirements must not force the entire daemon to run as root.

Still to define:

- exact systemd unit contents;
- restart policy;
- startup ordering relative to PipeWire, WirePlumber, and OpenRazer;
- failure backoff behavior;
- shutdown timeout;
- default daemon log filter and service environment configuration;
- package-time service enablement behavior.

### udev

`udev` rules may be required for:

- USB/HID permissions;
- stable device access;
- hotplug handling.

This is especially relevant to the Ajazz keyboard.

### XDG

RigCtl will follow XDG Base Directory conventions.

Configuration uses:

```text
$XDG_CONFIG_HOME/rigctl/
```

with `~/.config/rigctl/` as the default when `XDG_CONFIG_HOME` is not set. `config.toml` and the separate mouse, audio, and keyboard profile files are user configuration.

Persistent daemon state that should survive restarts but is not intended for manual maintenance uses:

```text
$XDG_STATE_HOME/rigctl/
```

with `~/.local/state/rigctl/` as the default. Possible state includes the last successfully applied profile for each subsystem and other daemon state that must survive a restart. This state is advisory rather than authoritative hardware state. The exact state schema remains undecided.

Transient runtime files use:

```text
$XDG_RUNTIME_DIR/rigctl/
```

The IPC socket remains:

```text
$XDG_RUNTIME_DIR/rigctl/rigctld.sock
```

Runtime files must not be treated as persistent configuration or state.

RigCtl should not create cache files unless a real need appears. If caching is added later, it will use `$XDG_CACHE_HOME/rigctl/`, with `~/.cache/rigctl/` as the default.

RigCtl does not currently require a user data directory. `$XDG_DATA_HOME/rigctl/` should be introduced only if persistent user-specific data appears that is neither configuration nor runtime state.

---

## Packaging

The primary target is **Omarchy / Arch Linux**.

Initial packaging will use Arch Linux tooling.

Conceptual package:

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

RigCtl will not implement its own package/module manager. Installation and removal remain the responsibility of the operating system package manager.

The service file is part of normal RigCtl installation. The exact package-time enablement mechanism and `systemctl --user` instructions remain to be defined, but the required end state is automatic daemon startup on future user logins after installation and configuration.

---

## Rust Workspace Structure

RigCtl uses a virtual Cargo workspace with three first-party crates. These crate boundaries are finalized for the initial implementation:

```text
crates/
├── rigctl/
├── rigctld/
└── rigctl-ipc/
```

The project deliberately avoids a generic `rigctl-core` crate, one crate per subsystem, plugin-style abstractions, and premature crate fragmentation. A new crate should be introduced only for a concrete need such as independent reuse, dependency isolation, separate testing, or clean protocol ownership.

### Repository layout

The recommended initial layout is:

```text
RigCtl/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── LICENSE
├── .gitignore
│
├── crates/
│   ├── rigctl/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── client.rs
│   │       ├── output.rs
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── status.rs
│   │           ├── mouse.rs
│   │           ├── keyboard.rs
│   │           └── audio.rs
│   │
│   ├── rigctld/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── app.rs
│   │   │   ├── logging.rs
│   │   │   ├── ipc/
│   │   │   │   ├── mod.rs
│   │   │   │   └── server.rs
│   │   │   ├── config/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── paths.rs
│   │   │   │   └── profiles.rs
│   │   │   ├── state/
│   │   │   │   └── mod.rs
│   │   │   └── subsystems/
│   │   │       ├── mod.rs
│   │   │       ├── mouse/
│   │   │       │   ├── mod.rs
│   │   │       │   ├── openrazer.rs
│   │   │       │   ├── capabilities.rs
│   │   │       │   └── profiles.rs
│   │   │       ├── keyboard/
│   │   │       │   ├── mod.rs
│   │   │       │   └── profiles.rs
│   │   │       └── audio/
│   │   │           ├── mod.rs
│   │   │           ├── profiles.rs
│   │   │           ├── eq/
│   │   │           │   └── mod.rs
│   │   │           └── crusher/
│   │   │               └── mod.rs
│   │   └── tests/
│   │       └── ...
│   │
│   └── rigctl-ipc/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── request.rs
│           ├── response.rs
│           ├── error.rs
│           ├── event.rs
│           └── version.rs
│
├── docs/
│   ├── architecture.md
│   ├── technical-stack.md
│   ├── configuration.md
│   ├── ipc.md
│   ├── development.md
│   └── hardware/
│       ├── mouse.md
│       ├── keyboard.md
│       └── audio.md
│
├── packaging/
│   ├── systemd/
│   │   └── rigctld.service
│   ├── udev/
│   │   └── ...
│   └── arch/
│       └── PKGBUILD
│
└── test-data/
    ├── profiles/
    └── protocol-fixtures/
```

The exact source-file set may evolve, but the initial crate boundaries and ownership responsibilities are intentional.

### Crate responsibilities

`rigctl` is the user-facing CLI client. It parses arguments and primitive types, connects to the Unix socket, serializes requests, deserializes responses, formats human and JSON output, and maps daemon errors to process exits. It must not contain hardware/backend implementation logic or directly depend on OpenRazer, OpenRazer-facing `zbus`, HID, PipeWire, Ajazz, or Crusher protocol code.

`rigctld` owns application behavior: the IPC server, configuration and profiles, persistent state, logging, subsystem initialization, hardware control, capability validation, write verification, backend error mapping, and future event delivery. Hardware-specific dependencies belong on the daemon side.

`rigctl-ipc` is the only first-party crate shared directly by both executables. It defines `Request`, `Response`, `RigCtlError`, `Event`, and `ProtocolVersion`. It contains data contracts only: it does not open sockets, run the server, access hardware, parse configuration, or implement subsystem behavior.

```text
              rigctl-ipc
              ▲        ▲
              │        │
          rigctl     rigctld
```

### Transport ownership

The shared crate defines what crosses the connection, while the executables own transport behavior:

- `crates/rigctl/src/client.rs` connects to `rigctld.sock`, sends requests, reads responses, and handles client-side transport failures;
- `crates/rigctld/src/ipc/server.rs` binds the socket, accepts clients, parses and dispatches requests, returns responses, and later sends subscribed events.

### Dependency direction

The intended direction is:

```text
                    rigctl
                       │
                       ▼
                  rigctl-ipc
                       ▲
                       │
                    rigctld
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
      Mouse         Keyboard         Audio
        │              │              │
    OpenRazer       Ajazz lib    PipeWire +
                                Crusher lib
```

Dependencies such as `rigctl-ipc → rigctld`, a subsystem depending on the CLI, or a reusable Ajazz/Crusher protocol library depending on RigCtl IPC or CLI types must not be introduced.

### Daemon and subsystem layout

Mouse, keyboard, and audio begin as modules under `rigctld/src/subsystems/`, not as `rigctl-mouse`, `rigctl-keyboard`, or `rigctl-audio` crates. Extraction is justified only by a real reuse or dependency-isolation need.

The daemon contains both `src/main.rs` and `src/lib.rs`. `main.rs` remains thin, while the library owns the application, IPC, configuration, state, and subsystem behavior. This allows most behavior to be tested without spawning the production executable; daemon integration tests live under `crates/rigctld/tests/`.

Configuration belongs to `rigctld`. The daemon alone parses and mutates `$XDG_CONFIG_HOME/rigctl/config.toml` and `$XDG_CONFIG_HOME/rigctl/profiles/`; the CLI requests configuration operations through IPC and does not independently interpret or modify these files.

### External protocol repositories

Ajazz and Crusher ANC 2 research may remain in separate repositories containing notes, sanitized fixtures or captures, protocol documentation, experimental utilities, and eventually reusable Rust libraries. Once reusable code exists, `rigctld` may depend on it directly. A Git dependency is preferred over a Git submodule unless a concrete future need justifies submodules; the exact mechanism remains undecided until reusable code exists.

Conceptual repository names are `Sajtisan/ajazz-ak820-max-protocol` and `Sajtisan/crusher-anc2-protocol`, alongside `Sajtisan/RigCtl`.

### Workspace manifest and lockfile

The root is a virtual workspace. Its conceptual manifest is:

```toml
[workspace]
resolver = "3"
members = [
    "crates/rigctl",
    "crates/rigctld",
    "crates/rigctl-ipc",
]

[workspace.package]
edition = "2024"
version = "0.1.0"
license = "MIT"
repository = "https://github.com/Sajtisan/RigCtl"

[workspace.dependencies]
serde = { version = "...", features = ["derive"] }
serde_json = "..."
thiserror = "..."
tracing = "..."
```

Exact dependency versions will be selected during repository setup. Because RigCtl is primarily an application workspace, `Cargo.lock` is committed for reproducible development and packaged builds.

---

## Resource Usage Strategy

Principles:

1. Use one lightweight daemon rather than multiple subsystem daemons.
2. Start the daemon automatically with the user session and keep it mostly idle when no work is required.
3. Minimize polling.
4. Prefer event-driven mechanisms where practical.
5. Do not use Electron.
6. Do not run a browser frontend or local HTTP server solely for UI purposes.
7. The CLI should send a request, print the result, and terminate.
8. Long-running work should exist only where technically required.
9. Fan control will require continuous evaluation only after that subsystem is implemented.
10. Hardware-specific protocol libraries should remain focused and small where practical.

---

## Logging and Diagnostics

RigCtl uses Rust's structured `tracing` ecosystem:

```text
rigctld
   │
   ▼
tracing events
   │
   ▼
tracing-subscriber + tracing-journald
   │
   ▼
systemd journal
```

`journald` is the canonical destination for daemon logs. RigCtl does not maintain its own log files, log directory, or log rotation. In particular, it must not create `~/.local/state/rigctl/logs/` or place logs under the XDG cache directory.

Daemon logs can be inspected with:

```bash
journalctl --user -u rigctld
journalctl --user -u rigctld -f
```

### Log levels

The daemon uses the conventional levels with the following RigCtl meanings:

- `ERROR`: an operation or subsystem failed and cannot proceed normally;
- `WARN`: a degraded, recoverable, or suspicious condition occurred;
- `INFO`: lifecycle and significant state changes, including startup, shutdown, backend availability, device discovery, profile application, and configuration reloads;
- `DEBUG`: implementation detail useful for diagnosing device selection, backend calls, validation, and state transitions;
- `TRACE`: very detailed protocol activity, including raw HID or Bluetooth research traffic when explicitly enabled.

Normal `INFO` startup logging must be sufficient to diagnose which configuration was loaded, which backends initialized, and which expected devices are available without flooding the journal during idle operation.

Structured fields are preferred over details embedded only in free-form messages. Useful fields may include `subsystem`, `operation`, `device_kind`, `profile`, `backend`, `error_code`, and elapsed time. Hardware identifiers should appear only at `DEBUG` or `TRACE` when they materially help diagnosis.

### Filtering and verbosity

`tracing-subscriber` uses `EnvFilter`, with the standard `RUST_LOG` environment variable available for global or targeted filtering. For example:

```bash
RUST_LOG=debug rigctld
RUST_LOG=rigctld::mouse=trace,rigctld=info rigctld
```

A systemd user-service override may set `RUST_LOG` for persistent troubleshooting. The exact default filter and packaged override instructions remain to be finalized.

The CLI is quiet during normal successful operation. Command results go to stdout; errors, warnings, and diagnostics go to stderr. Conceptual `-v` and `-vv` levels may expose progressively more client-side diagnostics, but verbosity must never change or contaminate machine-readable stdout, including `--json` output.

### Sensitive and protocol data

Logs must not contain credentials, secrets, complete user configuration, or unnecessary personal identifiers. Raw HID or Bluetooth traffic is disabled by default, restricted to `TRACE`, and enabled only for explicit development or troubleshooting. Logs are transient diagnostic evidence; confirmed protocol knowledge belongs in the project's research documentation and fixtures.

Logging is separate from the shared error contract. A failure to emit a log must not replace, suppress, or alter the structured error returned to the client. User-facing errors remain concise, while backend-specific context can be recorded in diagnostics at an appropriate level.

---

## Error Model

All RigCtl subsystems map failures into one shared error model before they cross the daemon IPC boundary. Mouse, keyboard, audio, fan, profile, transport, and daemon-lifecycle failures therefore use the same envelope and stable vocabulary.

Every shared error contains a required `code` and `message`. In structured CLI output it appears under `error`, and the full IPC response wraps it in the versioned response envelope documented in the IPC section:

```json
{
  "error": {
    "code": "device_not_found",
    "message": "Razer Viper V3 HyperSpeed is not connected"
  }
}
```

`code` is the stable machine-readable identifier. `message` is a concise human-readable explanation and may evolve without changing the meaning of the code.

An error may also contain a `details` object for relevant structured context:

```json
{
  "error": {
    "code": "unsupported_value",
    "message": "The requested polling rate is not supported by this device",
    "details": {
      "requested_hz": 4000,
      "supported_hz": [125, 500, 1000]
    }
  }
}
```

Clients must not require optional detail fields unless those fields are explicitly documented as part of a specific operation's contract.

The initial shared code vocabulary is:

```text
daemon_unavailable
device_not_found
backend_unavailable
unsupported_operation
unsupported_value
permission_denied
invalid_profile
invalid_value
transport_error
timeout
verification_failed
conflict
protocol_mismatch
invalid_request
internal_error
```

The codes have these intended meanings:

- `daemon_unavailable`: the client cannot connect to or communicate with `rigctld`;
- `device_not_found`: the requested or required device is not present;
- `backend_unavailable`: an external subsystem such as OpenRazer, PipeWire, or a required kernel interface is unavailable;
- `unsupported_operation`: the selected device or subsystem does not implement the requested operation;
- `unsupported_value`: the operation exists, but the requested value is outside the device's supported discrete set or range;
- `invalid_value`: the supplied value is malformed or invalid independent of device capability;
- `invalid_profile`: a profile has malformed TOML, an unsupported schema version, unknown or missing fields, an invalid combination, or another schema/capability validation failure;
- `permission_denied`: access is blocked by operating-system, device, D-Bus, or service permissions;
- `transport_error`: communication with hardware or an external backend failed;
- `timeout`: an operation exceeded its allowed time;
- `verification_failed`: a write completed but read-back or other confirmation did not match the requested state;
- `conflict`: the request conflicts with current state or another active operation;
- `protocol_mismatch`: the client requested an IPC protocol version that the daemon does not support;
- `invalid_request`: a parseable request has an unknown method, a missing required parameter, or an invalid field combination;
- `internal_error`: an unexpected RigCtl failure that does not fit a more specific stable code.

Subsystem-specific Rust errors, raw D-Bus replies, HID failures, Bluetooth details, and other backend diagnostics are mapped to this vocabulary rather than exposed as the default client contract:

```text
backend or subsystem failure
            │
            ▼
   RigCtl shared error
            │
            ▼
      JSONL IPC reply
            │
            ├──► human CLI rendering
            └──► unchanged JSON error envelope
```

Human-readable CLI output remains concise, for example:

```text
Error: Razer Viper V3 HyperSpeed is not connected.
```

With `--json`, the CLI preserves the shared `{ "error": ... }` shape rather than exposing raw backend failures. Errors and diagnostics are written to stderr, while successful results remain on stdout.

Process exit codes intentionally remain broad:

```text
0  success
1  runtime or operation failure
2  CLI usage or argument error
3  daemon unavailable
```

Most shared error codes map to exit status `1`; detailed meaning belongs in `error.code` and optional structured details rather than additional numeric exit codes.

An operation that only partially applies remains a failure. When useful, `details` may report applied work, failed work, and rollback status, but RigCtl must not imply transactionality that the backend or hardware cannot guarantee.

The JSON envelope, `error.code`, and exit-code meanings are stable machine-facing contracts. New codes should be added only for meaningfully distinct conditions, and existing codes must not be renamed casually. Diagnostic logging may contain deeper backend context, but it remains separate from the user-visible error and must not expose sensitive data.

---

## Testing Strategy

Testing is required throughout the project.

### Unit tests

Targets include:

- configuration parsing;
- schema-version and unknown-field rejection;
- profile filename validation and safe path resolution;
- required fields and schema-defined optional defaults;
- read operations that do not rewrite configuration;
- atomic replacement behavior for configuration writes;
- default-profile resolution and advisory active-state handling;
- profile validation;
- complete mouse-profile validation before writes;
- device/receiver capability selection;
- discrete polling-rate validation;
- DPI range/step validation once verified;
- write/read-back verification behavior;
- noise-control mode handling;
- future fan-curve calculations;
- command encoding/decoding;
- protocol parsing;
- protocol envelope, version, request-ID, and status validation;
- message-size enforcement and malformed-client isolation;
- exact event subscription and out-of-order response handling;
- shared error serialization and subsystem mapping;
- optional error-detail handling;
- process exit-code mapping;
- log filtering and sensitive-data redaction behavior.

### Mocked backend tests

Hardware-facing interfaces should be mockable where practical.

This includes OpenRazer mouse writes/profile application and the future Crusher ANC 2 transport once its protocol has been identified.

### Integration tests

The CLI/daemon contract should be testable independently of real hardware, including command parsing, `--json`, stdout/stderr separation, exit codes, structured errors, and daemon-unavailable behavior.

### Hardware-in-the-loop tests

Physical-device tests will be required for:

- OpenRazer behavior;
- Viper V3 HyperSpeed DPI, polling-rate, DPI-stage, profile application, and read-back behavior;
- Ajazz HID behavior;
- real fan-control writes after the fan subsystem is implemented;
- audio profile application;
- Crusher ANC 2 mode selection and state reporting.

Reverse-engineered Ajazz packets should be stored as reproducible fixtures where appropriate.

---

## Development Tools

Current planned environment:

```text
Operating System:    Omarchy / Arch Linux
Language:            Rust
Build System:        Cargo
Version Control:     Git
Repository:          GitHub
CLI:                 clap
Serialization:       serde
Configuration:       TOML
Config Schema:       version 1
Config Authority:    rigctld
Workspace:           virtual Cargo workspace
Shared IPC Types:    rigctl-ipc
Logging API:         tracing
Log Subscriber:      tracing-subscriber
Journal Integration: tracing-journald
Log Filtering:       RUST_LOG / EnvFilter
Normal Log Level:    INFO
Development Levels:  DEBUG / TRACE
CLI Diagnostics:     stderr
Persistent Logs:     none
Config Root:         $XDG_CONFIG_HOME/rigctl/
Profiles:            separate mouse/audio/keyboard TOML files
State:               $XDG_STATE_HOME/rigctl/
D-Bus:               zbus (external OpenRazer integration)
HID:                 hidapi / rusb
Fan Interface:       hwmon / sysfs (later)
Audio EQ:            PipeWire (integration mechanism TBD)
Crusher Control:     protocol TBD
IPC:                 Unix domain stream socket + UTF-8 JSONL
IPC Protocol:        version 1
IPC Message Limit:   1 MiB
Socket:              $XDG_RUNTIME_DIR/rigctl/rigctld.sock
Socket Permissions:  directory 0700 / socket 0600
Service Management:  systemd user service
Device Management:   udev
Packaging:           Arch PKGBUILD / pacman
Future UI:           Quickshell
```

Additional libraries will be selected after the remaining PipeWire integration, Crusher control protocol, and privilege designs are finalized.

---

## Prototyping

Python may be used temporarily during hardware reverse engineering.

```text
     Python HID experiment
               │
               ▼
       Send test command
               │
               ▼
   Observe keyboard response
               │
               ▼
       Confirm protocol
               │
               ▼
Implement final version in Rust
```

Python prototypes will not be part of the final runtime unless there is a strong technical reason to retain them.

Small standalone experimental tools are acceptable when they make protocol research safer or faster.

---

## Current Stack Summary

```text
Core runtime            Rust
CLI                     Rust + clap
Daemon                  Rust
Configuration           serde + TOML
Configuration schema    version 1
Configuration authority rigctld
Workspace               virtual Cargo workspace
Shared protocol crate   rigctl-ipc
Logging                 tracing + tracing-subscriber
Daemon log destination  systemd journal via tracing-journald

Mouse                   zbus + openrazer-daemon D-Bus
Keyboard                hidapi / rusb
Audio EQ                PipeWire; integration mechanism TBD
Crusher ANC 2           noise-control protocol TBD
Fans                    hwmon / sysfs (later; privilege model TBD)

IPC                     Unix domain stream socket + UTF-8 JSONL
IPC protocol            version 1
IPC message limit       1 MiB
Socket                  $XDG_RUNTIME_DIR/rigctl/rigctld.sock
Socket permissions      directory 0700 / socket 0600

Services                systemd user service; no initial socket activation
Device permissions      udev
Filesystem conventions  XDG
Config file             $XDG_CONFIG_HOME/rigctl/config.toml
Profiles                separate mouse/audio/keyboard TOML files
Profile identity        lowercase kebab-case filename
Persistent state        $XDG_STATE_HOME/rigctl/
Packaging               Arch Linux / pacman

Future UI               Omarchy / Quickshell
Development platform    Omarchy / Arch Linux
```

The exact libraries may change if hardware investigation or Linux integration requirements reveal needs that cannot be handled cleanly by the initially selected tools.
