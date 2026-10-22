# RigCtl IPC reference

## Purpose and ownership

This is the detailed reference for RigCtl's internal wire protocol. Keeping the
transport, message shapes, failure semantics, and future method contracts in one
place prevents clients and the daemon from developing different interpretations.
The [Technical Stack](./Technical_Stack_RigCtl.md#ipc) links here instead of
maintaining a second copy of the specification.

The protocol content was extracted from the existing Technical Stack. Future
contract changes should update this reference, its shared types and tests, and the
[decision log](./Development_Decisions_RigCtl.md) where the choice is significant.
This document does not select a new method API or implement any missing feature.

## Implementation status

This table describes the current working tree, including the local daemon
lifecycle implementation. It does not imply a released or merged feature.

| Layer | Current behavior |
| --- | --- |
| Shared message types | Request, success/error response, event, version constant, and error object are implemented in `rigctl-ipc` |
| Daemon transport components | Binding, accepting streams, JSONL framing, request decoding, and response writing are implemented as library operations |
| Client transport library | Sending requests, matching responses by ID, and buffering interleaved events are implemented; these client behaviors have no dedicated automated tests yet |
| Production daemon | Binds and owns the listener, then waits for shutdown; it does not call the accept/read/dispatch operations |
| Methods and subscriptions | No production methods, subscription handler, or event producer is implemented |
| User-facing CLI | Executable placeholder; it does not yet drive the client library through commands |

All method names and event exchanges below describe the intended protocol. They
are not working commands against the current daemon. Pretty-printed JSON is shown
for readability; an actual message must be encoded on one line and terminated
with a newline.

Current Rust types require JSON objects for `params`, `result`, `data`, and present
error `details`. Deserializing a request does not itself verify that its numeric
version is supported or that its method/parameters are semantically valid.
Unknown-field rejection is implemented for success/error response structures;
do not infer a universal unknown-field policy for every message from that.

RigCtl will use Inter-Process Communication (IPC) over a Unix domain stream socket between `rigctld`, the `rigctl` CLI, and the future Quickshell UI.

## Transport, framing, and access control

```text
Transport:       Unix domain stream socket
Location:        $XDG_RUNTIME_DIR/rigctl/rigctld.sock
Serialization:   JSON
Framing:         one JSON object per line
Encoding:        UTF-8
Message limit:   1 MiB
```

`rigctld` owns the socket. The `rigctl` CLI and future Quickshell UI connect as clients.

The documented and implemented listener permissions are:

```text
$XDG_RUNTIME_DIR/rigctl/                 0700
$XDG_RUNTIME_DIR/rigctl/rigctld.sock     0600
```

The current framing implementations count payload bytes before the terminating
newline against the 1 MiB limit. Connection lifetime and orderly socket cleanup
are described in the [developer guide](./Development_RigCtl.md#run-the-current-daemon).

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

## Protocol envelope and version

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

## Requests

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

## Responses

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

## Events and subscriptions

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

## Connection model

The protocol supports multiple outstanding requests on one connection. Responses are correlated by request ID, and clients must not assume that responses arrive in request order. Events may arrive between responses.

The CLI normally opens a short-lived connection, sends one request, receives its response, and disconnects. The future Quickshell UI may keep one persistent connection for requests and subscribed events. Both use the same protocol.

## Protocol failures

This is the required behavior for future production request handling. The current
lifecycle skeleton does not accept requests or generate these replies; its codec
only exposes decoding errors to its caller.

If a request uses an unsupported protocol version but can still be parsed well enough to recover its ID, the daemon returns `protocol_mismatch` in a normal v1 response envelope and reports its supported version in `details`.

If input is valid JSON and follows enough of the envelope to respond, but has a missing required parameter, an unknown method, or an invalid field combination, the daemon returns `invalid_request`.

Malformed JSON without a reliably recoverable request ID does not introduce a second error format. The daemon logs the condition, closes only that client connection, and remains running.

A message exceeding 1 MiB is handled the same way: the daemon logs the condition, closes the offending connection, performs no hardware operation from the partial request, and remains running.

## Rust representation

The shared `rigctl-ipc` crate currently defines `Request`, `Response`,
`SuccessResponse`, `ErrorResponse`, `Event`, and `RigCtlError`, with Serde
serialization. `RequestId` is an alias for `u64`, `JsonObject` is a JSON object map,
and version fields and `PROTOCOL_VERSION` use `u32`. A dedicated `ProtocolVersion`
type mentioned in earlier design prose is not implemented. The shared crate owns
data contracts, not socket transport.

## Communication layers

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

## Method documentation to add with dispatch

There are no implemented method entries yet. The names listed above establish the
planned naming vocabulary; many operation-specific parameter and result schemas
remain to be specified when their implementations are designed.

For each method, add an entry with this structure:

```text
Method name and implementation status:
Owning subsystem and handler:
Supported protocol version:
Purpose and prerequisites:
Params: required/optional fields, types, units, validation
Result: fields, types, authoritative versus advisory state
Errors: stable codes and any operation-specific details contract
State changes and hardware writes:
Verification and partial-failure behavior:
Concurrency, cancellation, and idempotency expectations:
Events emitted, if any:
Request, success, and failure examples:
Contract and integration tests:
```

Do not choose hardware-dependent parameter ranges or status fields merely to fill
this skeleton. Link verified [research findings](./hardware/README.md) before
turning them into capability requirements.

## Event documentation to add with producers

For each implemented event, record its exact name, data fields, triggering
condition, subscription requirement, initial-state behavior, and relevant tests.
Define delivery, ordering, and disconnect behavior when event infrastructure is
implemented; this skeleton does not invent those policies.

## Errors and client output

The response envelope above uses the shared error object. The initial code
vocabulary and meanings remain documented in the
[Error Model](./Technical_Stack_RigCtl.md#error-model). Current Rust error codes are
strings; a complete typed vocabulary and mapping policy are unfinished.

IPC responses and CLI output are different contracts. A future CLI prints the
requested successful object directly in JSON mode rather than echoing the full
IPC response envelope. See the
[CLI output contract](./Technical_Stack_RigCtl.md#output-contract); that rendering
is not implemented by the current executable.

## Tests and open work

See [protocol and transport tests](./tests/ipc.md) for exact implemented coverage.
In particular, fixture names do not prove a production method exists, and the
transport suite does not test a running request dispatcher.

Work remaining includes production request dispatch, recoverable-envelope error
handling, protocol-version validation, operation-specific schemas, event
subscription/production, and end-to-end client/daemon tests. Document additional
connection limits, timeout policy, duplicate-ID handling, or compatibility rules
when chosen; none is newly selected by this reference skeleton.
