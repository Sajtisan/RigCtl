# Razer Viper V3 HyperSpeed research

**Status:** Research skeleton. No target-device experiments are recorded here.

## Importance and documented direction

The mouse backend is planned to communicate with OpenRazer over D-Bus. Reliable
control requires the actual device and receiver capabilities, not values inferred
from another Razer model. These notes will support capability checks and explain
why a setting is accepted, rejected, or omitted.

The [Technical Stack](../Technical_Stack_RigCtl.md#mouse-subsystem) describes named
Linux-side RigCtl profiles applied to one active onboard hardware configuration.
Do not interpret multiple profile files as multiple onboard slots. Persistence of
individual OpenRazer settings still needs experimental evidence.

## Setup and identification

To record: mouse and receiver identity, firmware, connection mode, kernel,
OpenRazer driver/daemon versions, D-Bus discovery results, permissions, and tools.
Do not fill in USB IDs, interface paths, or firmware values until observed.

## Capability questions

| Area | Evidence needed | Status |
| --- | --- | --- |
| Discovery and identity | Match the intended mouse and receiver to the selected backend device | Not investigated here |
| DPI | Readable values, valid range/step, X/Y behavior, write/read-back behavior | Not investigated here |
| Polling rate | Verified discrete values for this receiver and connection | Not investigated here |
| DPI stages | Supported stage count, values, active-stage behavior, backend representation | Not investigated here |
| Battery | Available fields, meaning, update behavior, unavailable-state handling | Not investigated here |
| Additional settings | Read/write/read-back evidence before adding fields such as lift-off distance | Not investigated here |
| Persistence | Distinguish daemon restart, backend restart, reconnect, and device power cycle | Not investigated here |

## Experiments and evidence

No entries yet. Add `MOUSE-...` records using the
[experiment template](./README.md#experiment-record-template), including read-back
and recovery behavior. Link sanitized D-Bus observations or fixtures when available.

## Backend and profile mapping

To grow: map a verified operation to its backend call, argument types, capability
validator, public error mapping, read-back rule, and profile field. Keep unverified
fields outside the accepted schema. Describe best-effort restoration and which
settings cannot be rolled back reliably.

## Hardware validation procedure

Not defined yet. Once experiments establish supported behavior, add prerequisites,
baseline settings, one operation per test, verification, restoration, and the exact
manual or automated command. Current automated tests do not access the mouse.
