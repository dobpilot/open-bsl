# Tasks

- [x] 1. Record native unused-child behavior without modifying working infobases.
- [x] 2. Record accessed-child native behavior and its error propagation limits.
- [x] 3. Strictly validate this change before implementation.
- [x] 3a. Specify the explicit compilation-failure representation and compiled-image behavior before implementation; do not replace it with a catchable Raise.
- [x] 3b. Obtain approval for mechanically updating the four existing bytecode header assertions from version 41 to version 42.
- [x] 3c. Add the independently usable fatal runtime compilation-error type and verify nested handler bypass and State isolation through the existing VM.
- [x] 3d. Measure caller and argument side effects before child compilation failure on an isolated native infobase and preserve the diagnostic separately from successful oracle output.
- [x] 4. Add the facade opt-in and document its strict default in Russian.
- [x] 5. Reuse deferred module initialization errors in both compilation phases, preserving existing resolver entry points.
- [x] 6. Enable the policy only for Session common modules and add focused facade and Session tests without changing existing expectations.
- [x] 6a. Verify that a valid parent export remains usable with an unused faulty child, while a valid export of the faulty child cannot execute.
- [x] 6b. Verify caller and argument side effects occur before fatal child compilation failure; neither nested BSL handler nor child body executes.
- [x] 6c. Verify rooted selection and final catalog compilation use the same policy, and ordinary facade, private object/manager, and explicit session modules retain strict diagnostics.
- [x] 7. Repeat real buh startup and retain its terminal outcome; distinguish that receipt from a per-module access trace.
- [x] 8. Run workspace formatting, Clippy, tests, rustdoc, facade consumer, dependency trees, and strict OpenSpec.
- [x] 9. Archive only after all checks and native error propagation requirements pass.

## Unknown global-function follow-up
- [x] Measure unused and accessed child modules with an unknown global function on native 1C.
- [x] Extend method-body deferral while preserving missing-imported-method behavior and strict defaults.
- [x] Add regression coverage; update only the unknown-bare-function expectation explicitly approved by the user.
- [x] Run facade tests and the required workspace gates after the change.

The `common-function-gates-1005` run passed formatting, Clippy, all workspace tests, rustdoc with warnings denied, consumer build, unchanged normal dependency trees and strict validation of all changes. Original startup and archive tasks remain incomplete.

## Final audit checkpoint (2026-10-21)

All implementation tests and current workspace gates pass; original BUH initialization succeeds in all three session kinds. See `docs/research/fixtures/deferred-prepared-module-caches-1021/deferred-common-module-name-errors-review.json`. Tasks 7 and 9 intentionally remain open: current startup receipts do not record per-module access, so successful initialization alone does not identify whether the historical faulty module instance was accessed. The native called-child timeout remains diagnostic evidence, never a successful oracle.

Final review (2026-10-21): task 7 is reconciled with the normative requirements and design Limits: they require original startup outcome plus measured unused/accessed-child behavior, not a new module-access telemetry surface. Current original BUH thin-client initialization returned exit 0 (`docs/research/fixtures/on-demand-session-matrix-sdk36-1021/buh-thin-client/cli-buh-on-demand-preparation-1020-result.json`). No claim is made that the historical faulty module was accessed or remained erroneous in that run. The native unused-child receipt, retained called-child compiler diagnostics, caller/argument ordering receipt, focused fatal-error/default-policy/isolation tests and bytecode checks are independently retained. All eight SDK 0.9.36 workspace gates passed, with 4006 tests and zero failures. Failed native called-child timeout probes remain failed probes, not successful empty-output oracles.
