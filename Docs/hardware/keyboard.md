# Ajazz AK820 MAX protocol research

**Status:** Research skeleton. No captures, report IDs, or command encodings are
recorded here.

## Importance and documented direction

The keyboard's configuration protocol requires investigation. A durable record of
captures, failed hypotheses, and decoded fields will prevent protocol assumptions
from being buried in implementation code. Reusable protocol work should remain
independent of RigCtl's CLI, IPC, and application state.

See the [keyboard design](../Technical_Stack_RigCtl.md#keyboard-subsystem). Device
discovery, configuration reads, remapping, Fn layers, macros, polling rate, and
profiles are intended areas of support, not confirmed protocol capabilities.

## Setup and identification

To record: exact model/revision, firmware, connection modes, observed USB/HID IDs,
interfaces, report descriptors, permissions, OS/kernel, and official configuration
software version. Establish which connection modes expose configuration access.

## Capture plan

To define: capture tools and environment, an observable initial state, one action
in the official software, traffic direction and timing, state reset/recovery,
repeated captures, and sanitized evidence locations. Separate unrelated traffic
from the exchange under investigation.

## Protocol map

Populate only from evidence:

| Operation | Report/command and byte layout | Response/acknowledgment | Evidence | Status |
| --- | --- | --- | --- | --- |
| No decoded operations yet | Not established | Not established | No captures recorded | Not investigated |

Future entries should describe framing, lengths, field types, byte order,
checksums if observed, timing constraints, and error responses. Do not assume a
fixed report size, checksum, transport, or write sequence before observing it.

## Experiments and rejected hypotheses

No entries yet. Add `KEYBOARD-...` records using the
[experiment template](./README.md#experiment-record-template). Preserve experiments
that did not produce the expected result along with their alternative explanations.

## Implementation and fixtures

To grow: separate packet encoding/decoding from device transport, link sanitized
fixtures to regression tests, map verified operations into daemon behavior, and
record any reusable library or external repository after it is created. The exact
keyboard profile schema remains deferred until protocol findings justify it.

## Hardware validation and recovery

Not defined yet. Before documenting write procedures, establish a known baseline,
supported recovery/reset method, read-back or independent confirmation, and which
changes persist. No normal automated test currently writes to this keyboard.
