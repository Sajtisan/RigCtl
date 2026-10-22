# Fan-control research

**Status:** Deferred. This skeleton does not select an interface, privilege model,
fan curve, or control-loop policy.

## Importance and scope boundary

Fan control requires verified hardware mappings and explicit failure behavior.
These notes will preserve the evidence behind sensor selection, permitted writes,
and safe fallback decisions before a continuous controller is implemented.

The project explicitly defers fans until the initial foundations and subsystems
are stable. Future fan requirements must not make the entire user daemon run as
root. See the [fan plan](../Project_Plan_RigCtl.md#later--fan-controller).

## Target inventory

Not recorded yet. Identify the actual motherboard/controller, kernel driver,
hwmon devices, fan channels, temperature sources, and relationships between them.
Distinguish a discovered path from a verified physical mapping.

## Read and write capability evidence

To grow: available telemetry, units, writable controls, mode transitions,
hardware/firmware behavior, value limits, permissions, and recovery to the
pre-experiment control mode. Begin with observation; no write commands are
provided by this skeleton.

## Privilege-model evaluation

Pending target inspection. Compare concrete approaches only after the necessary
operations are known. Document the minimum access each needs, which process owns
it, how ownership conflicts are handled, and how access is installed and revoked.
No helper, udev rule, or sysfs-permission mechanism is chosen here.

## Control and failure questions

To decide later: fan/sensor association, temperature-to-speed curve format,
interpolation, evaluation interval, hysteresis, startup state, missing sensors,
failed writes, concurrent controllers, daemon failure, and shutdown restoration.
Record the intended fallback and how it is verified for each relevant failure.

## Experiments and validation

No entries yet. When fan research begins, use `FANS-...` records following the
[experiment template](./README.md#experiment-record-template), with a baseline and
recovery procedure established before controlled writes. Define separate physical
validation instructions before treating the subsystem as usable.
