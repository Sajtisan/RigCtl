# PipeWire EQ integration research

**Status:** Integration research skeleton. No EQ hosting mechanism or prototype
results are selected by this document.

## Importance and documented direction

PipeWire and a parametric-EQ profile model are selected project directions. The
mechanism that hosts the EQ, stable filter fields, and validation limits remain
undecided. Recording prototype results will explain the eventual choice and its
behavior across output changes and restarts.

EQ processing runs on the Linux host and is independent of the headset's ANC,
Off, and Stay-Aware modes. See the
[audio design](../Technical_Stack_RigCtl.md#audio-subsystem).

## Evaluation environment

To record: PipeWire and session-manager versions, output identities, routing and
graph state, sample-rate assumptions, selected test signal/material, and how the
unmodified signal path is restored. No measured latency or resource cost is
claimed yet.

## Candidate integration mechanisms

Research actual mechanisms before filling this comparison:

| Candidate | Profile mapping | Output selection and reconnects | Control and cleanup | Measured cost | Evidence |
| --- | --- | --- | --- | --- | --- |
| No candidate evaluated here | Open | Open | Open | Not measured | None recorded |

The eventual comparison should include ownership of processing nodes, interaction
with the desktop audio session, update behavior, failure visibility, and whether
multiple controllers can interfere.

## Profile and state questions

To establish: supported filter types and fields, units and limits, gain/headroom
behavior, complete-profile validation, active-profile observation, default-profile
application, reconnect behavior, and unavailable-output handling. Do not treat
conceptual profile examples as an implemented schema.

## Experiments and evidence

No entries yet. Add `EQ-...` records using the
[experiment template](./README.md#experiment-record-template). Record graph
observations, objective measurements where useful, audible observations separately,
and exact removal/restoration steps.

## Decision and validation procedure

Pending. Once a prototype is reproducible, record the selected mechanism and
tradeoffs in the decision log, define its schema in the appropriate reference,
and add mocked tests plus explicit host-audio integration instructions. Current
automated tests neither configure PipeWire nor alter audio routing.
