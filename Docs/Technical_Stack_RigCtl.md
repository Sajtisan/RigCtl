# RigCtl — Technical Stack

## Overview

RigCtl is designed as a lightweight Linux hardware control system with a strong focus on:

- minimal resource usage;
- modularity;
- direct hardware access;
- CLI-first development;
- Linux-native integration;
- reusable hardware-specific modules.

The application will primarily be implemented in **Rust**.

The browser-based interface will be an optional frontend and will not be required for the hardware controllers to function.

---

## Backend

### Rust

Rust will be used for:

- the core application;
- the CLI;
- hardware communication;
- module implementations;
- configuration handling;
- the local HTTP API;
- background services where required.

Rust was chosen because RigCtl interacts directly with Linux hardware interfaces and should have a very small runtime footprint.

It provides:

- memory safety without garbage collection;
- low runtime overhead;
- native binaries;
- good Linux systems-programming support;
- strong type safety;
- good support for CLI applications;
- easy separation into reusable libraries.

---

## CLI

### clap

The command-line interface will use the `clap` crate.

Example commands:

```bash
rigctl mouse status
rigctl mouse dpi 800

rigctl keyboard keymap get
rigctl keyboard remap capslock escape

rigctl fans profile apply silent
```

The CLI will be the primary interface during early development.

All hardware functionality should be usable without the browser interface.

---

## Configuration

### serde + TOML

Configuration files will be serialized using `serde`.

TOML will be used as the main human-readable configuration format.

Example:

```toml
[mouse]
dpi = 800
polling_rate = 1000

[keyboard]
profile = "default"

[fans.profiles.silent]
curve = [
    [30, 20],
    [45, 30],
    [60, 50],
    [75, 75],
    [85, 100]
]
```

The CLI and browser interface will operate on the same underlying configuration structures.

---

## Mouse Module

### OpenRazer

The Razer Viper V3 HyperSpeed will be controlled through OpenRazer.

The RigCtl mouse module will communicate directly with the OpenRazer service instead of relying on Polychromatic.

#### Planned communication

```text
     RigCtl
        │
        ▼
      D-Bus
        │
        ▼
OpenRazer Daemon
        │
        ▼
  Razer Device
```

### zbus

The Rust `zbus` crate will be used for D-Bus communication.

The mouse-specific implementation will remain isolated in its own crate.

```text
rigctl-openrazer
```

---

## Keyboard Module

The Ajazz AK820 MAX module will communicate directly with the keyboard using USB HID.

Since the configuration protocol is undocumented, this module will also contain the results of the reverse-engineering process.

### Possible libraries

#### hidapi

`hidapi` will be the preferred initial solution for HID communication.

It provides a relatively simple interface for:

- device discovery;
- HID reports;
- feature reports;
- reading and writing HID data.

#### rusb

If lower-level USB communication becomes necessary, the `rusb` crate may be used.

The exact implementation will depend on how the AK820 MAX communicates with the official software.

The expected architecture is:

```text
rigctl-ajazz
      │
      ├── device discovery
      ├── HID transport
      ├── protocol implementation
      └── configuration structures
```

The protocol implementation should not depend on the RigCtl user interface.

This makes it possible to extract or reuse the implementation in another Linux project later.

---

## Fan Module

The fan module will interact with Linux hardware interfaces.

The main target will be:

```text
/sys/class/hwmon/
```

The module will provide:

- fan profiles;
- fan curves;
- profile persistence;
- curve validation;
- applying fan curves.

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

The CLI and GUI will use the same internal curve representation.

---

## Fan Service

Most RigCtl modules should not require permanently running background processes.

The fan controller may be an exception if fan curves need to be evaluated continuously.

Possible architecture:

```text
 rigctl-fans.service
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
        Sleep
```

The service should remain very small and only perform the operations necessary for fan control.

---

## Local API

### Axum

The browser frontend will communicate with RigCtl through a local HTTP API implemented using `axum`.

Example endpoints:

```text
GET  /api/modules

GET  /api/mouse
POST /api/mouse/dpi

GET  /api/keyboard/keymap
PUT  /api/keyboard/keymap

GET  /api/fans/profiles
POST /api/fans/profiles
POST /api/fans/profiles/{profile}/apply
```

The API will only be accessible locally.

Example address:

```text
http://127.0.0.1:<port>
```

---

## Real-Time Data

### Server-Sent Events

Server-Sent Events may be used for continuously changing values such as:

- mouse battery status;
- fan state;
- temperature values;
- hardware status changes.

Example:

```text
GET /api/events
```

SSE is preferred over WebSockets unless bidirectional real-time communication becomes necessary.

---

## Browser Interface

The graphical interface will run in the user's existing browser.

No Electron or permanently running desktop GUI will be used.

### Frontend stack

The planned frontend stack is:

```text
TypeScript
Preact
Vite
HTML
CSS
```

Preact provides a component-based frontend while remaining lightweight.

If the interface remains sufficiently simple, plain TypeScript may be used instead.

The browser interface should contain as little business logic as possible.

Its responsibility is primarily:

```text
  User input
       │
       ▼
   Local API
       │
       ▼
RigCtl backend
```

---

## GUI Lifecycle

The graphical interface does not need to run continuously.

It can be launched using:

```bash
rigctl ui
```

The command can:

1. start the local HTTP server;
2. serve the frontend files;
3. open the default browser;
4. provide access to the RigCtl API.

When the interface is no longer required, the local UI service can terminate.

This allows most RigCtl functionality to have zero idle resource usage.

---

## Linux Integration

### systemd

`systemd` will be used for components that genuinely require background execution.

For example:

```text
rigctl-fans.service
```

Other modules should avoid permanent services where possible.

---

### udev

`udev` rules may be required for:

- USB device permissions;
- HID access;
- device detection;
- hotplug handling.

This will be especially relevant for the Ajazz keyboard module.

---

## Packaging

RigCtl is primarily targeted at Omarchy / Arch Linux.

The project will therefore use Arch Linux packaging.

Possible packages:

```text
rigctl
rigctl-mouse-openrazer
rigctl-keyboard-ajazz
rigctl-fans
rigctl-web
```

Only selected modules should be installed.

Example:

- [x] RigCtl Core
- [x] Mouse Controller
- [x] Keyboard Controller
- [ ] Fan Controller

The package manager should remain responsible for the actual installation and removal of files.

RigCtl should not attempt to replace the operating system's package manager.

---

## Module Structure

The project will use a Rust workspace.

```text
rigctl/
├── Cargo.toml
│
├── crates/
│   ├── rigctl-core/
│   ├── rigctl-cli/
│   ├── rigctl-api/
│   ├── rigctl-openrazer/
│   ├── rigctl-ajazz/
│   └── rigctl-fans/
│
├── web/
│   ├── src/
│   ├── index.html
│   └── package.json
│
├── packaging/
│   └── arch/
│
└── docs/
    ├── architecture.md
    ├── tech-stack.md
    └── ajazz-protocol.md
```

Hardware-specific crates should have as few dependencies on the RigCtl core as possible.

For example:

```text
rigctl-ajazz
      │
      ▼
  HID / USB
```

is preferred over:

```text
rigctl-ajazz
      │
      ▼
 RigCtl Core
      │
      ▼
   Web API
      │
      ▼
  Frontend
```

This keeps the hardware implementation reusable.

---

## Resource Usage Strategy

Minimal resource usage is a core design requirement.

The following rules will guide development:

1. Hardware modules should not run continuously unless necessary.
2. The browser interface should only run when requested.
3. No Electron runtime will be used.
4. CLI operations should start, perform their task and terminate.
5. Separate daemons should only exist when continuous monitoring is required.
6. Disabled modules should not be installed.
7. Hardware-specific libraries should remain small and independent.

In normal operation, components such as mouse and keyboard configuration should consume no resources when they are not actively being used.

---

## Development Tools

Planned development environment:

```text
Operating System:    Omarchy / Arch Linux
Language:            Rust
Build System:        Cargo
Version Control:     Git
Repository:          GitHub
Frontend:            TypeScript + Preact
Frontend Build:      Vite
API:                 Axum
CLI:                 clap
Serialization:       serde
Configuration:       TOML
D-Bus:               zbus
HID:                 hidapi / rusb
Service Management:  systemd
Device Management:   udev
Packaging:           Arch PKGBUILD / pacman
```

---

## Prototyping

Python may be used temporarily during reverse engineering.

For example:

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

Python prototypes will not be part of the final runtime unless there is a strong technical reason to keep them.

---

## Final Stack

```text
Core / Hardware       Rust
CLI                   Rust + clap
Configuration         serde + TOML

Mouse                 zbus + OpenRazer
Keyboard              hidapi / rusb
Fans                  hwmon / sysfs

HTTP API              Axum
Real-time updates     SSE

Frontend              TypeScript + Preact + Vite

Services              systemd
Device permissions    udev

Packaging             Arch Linux / pacman
Development platform  Omarchy
```

The exact libraries may change during development if hardware investigation reveals requirements that cannot be handled cleanly by the initially selected tools.
