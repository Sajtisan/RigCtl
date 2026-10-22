# Hardware and backend research

## Purpose

These notes preserve experiments, protocol evidence, and unresolved questions
before they become production behavior. They are particularly important where a
device protocol is undocumented or a backend exposes capabilities that vary by
device, receiver, firmware, or connection mode.

The project plan defines intended support. A research note establishes what was
actually observed and under which conditions. A planned feature, vendor UI option,
or packet interpretation is not proof that RigCtl can implement it reliably.

## Research index

| Target | Current record | Main question |
| --- | --- | --- |
| [Razer Viper V3 HyperSpeed](./mouse.md) | Research skeleton; no experiments recorded here | Which OpenRazer operations and value constraints are verified on the target mouse/receiver? |
| [Ajazz AK820 MAX](./keyboard.md) | Research skeleton; no captures or decoded packets recorded here | Which HID/USB exchange controls each supported feature? |
| [PipeWire EQ](./pipewire-eq.md) | Integration research skeleton | Which mechanism should host and control the parametric EQ? |
| [Skullcandy Crusher ANC 2](./crusher-anc2.md) | Research skeleton; control channel unverified | How are noise-control modes queried and selected from Linux? |
| [Fans](./fans.md) | Explicitly deferred research skeleton | Which real hwmon interface and minimum permissions are appropriate? |

These statements describe what is recorded in this repository, not whether
experiments exist elsewhere. No subsystem development order is chosen here.

## Evidence labels

Use these labels for findings rather than treating every note as a specification:

- **Confirmed:** Reproduced on an identified setup with linked observations or
  fixtures. State the conditions and limits; one device revision is not all devices.
- **Hypothesis:** A possible interpretation with the experiment needed to test it.
- **Inconclusive:** An experiment produced insufficient or conflicting evidence.
- **Disproved:** A hypothesis failed; retain the result to avoid repeating it.
- **Not investigated:** No experiment is recorded for this question.

Product choices belong in the [decision log](../Development_Decisions_RigCtl.md).
Promote a hardware finding into the Technical Stack, capability validation, and
tests only when the evidence supports that scope.

## Experiment record template

Use a stable ID within the target note, such as `MOUSE-001` or `KEYBOARD-001`.
An ID in an example is a naming pattern, not a completed experiment.

```text
ID and date:
Question or hypothesis:
Status: confirmed / hypothesis / inconclusive / disproved
Device, receiver, firmware, connection mode:
OS/kernel, backend/driver, tool versions, RigCtl revision:
Initial state and recovery method:
One controlled change:
Reproduction steps:
Expected observation:
Actual observation, including failed attempts:
Capture or sanitized fixture path and relevant offsets:
Interpretation and alternative explanations:
Read-back or independent confirmation:
Persistence after reconnect/restart/power cycle, if relevant:
Known limits and next experiment:
Implementation issue, regression test, or decision links:
```

## Capture and fixture handling

Record capture provenance, direction, transport framing, timestamps where useful,
and the conditions needed to interpret it. Keep conclusions separate from raw
observations. Describe any redactions that affect reproducibility.

Do not commit pairing keys, credentials, unrelated application traffic, or personal
identifiers that are unnecessary to the research. Keep sensitive originals outside
the repository and link only sanitized evidence suitable for sharing.

No capture directory or new protocol repository is created by these skeletons.
Choose locations when evidence exists. Reusable automated fixtures can follow the
planned `test-data/protocol-fixtures/` convention, or live beside their consuming
tests when more appropriate. Any future external protocol repository should be
linked explicitly after it exists.

## Adding a supported operation

Document discovery and identity, permitted values, the operation itself, its
observable result, failure behavior, persistence, and a recovery path. A successful
write alone is not read-back verification. For multi-setting profiles, validate the
complete request before writes and record what can be restored after a partial
failure.

Keep physical experiments separate from the normal
[hardware-independent test suite](../tests/README.md). Add explicit hardware test
instructions to the target note when those tests exist.
