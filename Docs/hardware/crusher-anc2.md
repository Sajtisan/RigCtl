# Skullcandy Crusher ANC 2 control research

**Status:** Research skeleton. The Linux-side control channel and protocol have
not been established in these notes.

## Importance and documented direction

RigCtl intends to select one of three exclusive noise-control modes: ANC, Off,
or Stay-Aware. Documenting control traffic and state observations will distinguish
requested mode from confirmed mode and explain what can be reported after a
reconnect or an external controller changes the headset.

The three-state product model is fixed; its encoding and communication mechanism
are not. Do not assume Bluetooth LE/GATT or invent identifiers before investigation.
See the [noise-control design](../Technical_Stack_RigCtl.md#crusher-anc-2-noise-control).

## Setup and identification

To record: model/revision, firmware, phone/app version when relevant, Linux
Bluetooth/audio environment, connection arrangement, existing controllers, and a
safe baseline mode. Keep pairing material and unnecessary personal identifiers
out of shared captures.

## Investigation questions

| Question | Status |
| --- | --- |
| Which transport and endpoint carry control commands? | Not investigated here |
| Can the current mode be queried, or only inferred after a command? | Not investigated here |
| How is each of ANC, Off, and Stay-Aware selected and acknowledged? | Not investigated here |
| Are unsolicited mode-change notifications available? | Not investigated here |
| What survives reconnects and device power cycles? | Not investigated here |
| What happens when another application controls the headset? | Not investigated here |
| What indicates unavailable, unknown, rejected, or timed-out state? | Not investigated here |

## Captures and experiments

No entries yet. Add `CRUSHER-...` records using the
[experiment template](./README.md#experiment-record-template). Capture one mode
transition at a time and distinguish observed messages from their proposed
interpretation. Link sanitized fixtures only after the control exchange is isolated.

## Command and state model

To grow: verified transport identifiers, request/response encoding, notification
decoding, current-mode confidence, state invalidation, reconnect behavior, and
public error mapping. Keep host EQ profiles separate from noise-control state.
Adjustable ANC/Stay-Aware intensity remains later work; no ranges or CLI syntax
are selected here.

## Hardware validation and recovery

Not defined yet. Add reproducible mode-change checks, independent confirmation,
recovery to the baseline, and multi-controller tests after the protocol is known.
Current automated tests do not connect to or change the headset.
