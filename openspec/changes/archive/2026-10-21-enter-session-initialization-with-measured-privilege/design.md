# Design

## Evidence
`docs/research/fixtures/session-entry-privilege-1016/` retains the native input, raw oracle, commands, checksums and terminal receipts. InitialPrivilege is assigned only by the automatic Undefined branch; LazyPrivilege remains unset until a form reads it. Neither the module nor caller sets privileged mode explicitly. Observed values are No / Yes / No / Yes / No, followed by completed=YES.

## Boundary
The session component selects an explicit entry mode for its two automatic handler calls. The VM remains unaware of platform handler names and configuration module naming. Existing generic handler calls retain ordinary inherited context. The mode applies to the nested execution only; caller counters and the authenticated principal remain unchanged, including exception paths. Use existing security-mode machinery rather than temporary mutations of caller counters or thread-local state. Existing safe-mode suppression remains authoritative; this control does not measure additional safe-mode or background rules and does not justify overriding them.

The implementation must cover host initial entry and nested selective entry through the same generic execution mechanism. Common-module Privileged metadata consumption is separate and is not implemented here. Current BUH memory diagnostics continue with their frozen binary; no mixed-source success claim is permitted.

## Automatic entry counters
A second native control retained under the same fixture/counter-control observes that SetPrivilegedMode(false) without a local enable raises and leaves automatic privilege active. A local true/false pair succeeds and also leaves automatic privilege active, in both initial and selective entries. Therefore seed the child inherited snapshot, not its decrementable frame counter. Module body initialization retains its ordinary inherited snapshot; no measurement here establishes automatic privilege for that body.

## Background entry evidence
A third owned filebase control records automatic initial and selective handler entry in files named with the actual platform session number. The job session number differs from the parent, and the job-specific initial receipt proves entry independently of copied parameters. Both job handler entries observe privilege; ordinary job code before/after the selective read observes ordinary mode. Evidence is retained in fixture/background-control. WebClient entry remains unmeasured and must retain the three-part open-question discipline until a real web-client oracle is available.
