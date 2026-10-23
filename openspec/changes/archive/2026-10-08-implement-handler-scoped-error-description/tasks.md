## Evidence and design
- [x] Preserve the native ordinary-call/nested-handler control, provenance and hashes; verify the driver also terminates on the existing unsupported implementation.
- [x] Inspect the existing component stub, task exception value, compiler handler lowering and bytecode verification; document why the task-wide value and inferred jump layout are insufficient.
- [x] Select the existing exception record extended with an explicit handler end and per-frame caught snapshots; preserve eight additional native nonlocal-exit observations without an independent exception model.
- [x] Measure dynamic snippets and the relation to existing error presentations; preserve native receipts.
- [x] Preserve eight native async parent/child and await-path observations; distinguish observed await from proven pending suspension.
- [x] Measure object-body initialization and exported-method callbacks; replay all nine native observations through SessionFactory without further runtime changes.
- [x] Add deterministic facade/VM pending-resume coverage: block both native-control file operations with host channels and verify Waiting before each release.
- [x] Add new red regression tests for the 32 scope observations, aliases, arity and independent states; record both current stub failures without changing existing tests.
- [x] Add and strictly validate the explicit handler-extent serialized-format delta before implementation.

## Implementation and verification
- [x] Implement both existing function aliases through the current builtin path and per-frame snapshots; serialize format 43 explicit handler extents, validate and relocate their boundaries.
- [x] Add focused tests for empty scope, explicit messages, nested restoration, ordinary call isolation, nonlocal exits and independent states; two CLI tests and the proven-pending facade test pass.
- [x] Replay all 32 native scope observations without changing their output; fix the async driver to use an explicit async procedure and repeat the native acquisition before replay.
- [x] Verify optimized/plain corpus agreement (155.53 s), 7 existing compiler round trips, 26 malformed-image tests, and the open-questions registry without editing their expectations.
- [x] Complete broader existing error-info and rethrow regression coverage after the approved test-literal update.
  Existing facade suites `error_info`, `brief_error_description`, `dynamic_errors` and `error_notifications` pass unchanged on 2026-10-07; log: `/tmp/open-bsl-handler-existing-errors-1007.log`. VM unit coverage still awaits the test-literal approval.
  On 2026-10-08 the user explicitly approved all six edits in `/tmp/open-bsl-handler-test-update-proposal-1007.patch`: five handler-end fields and the textual handler-width assertion from four to five numbers for format 43. The patch was applied after `git apply --check`; all 286 VM tests passed (`/tmp/open-bsl-vm-tests-1008.log`). The user subsequently approved both `/tmp/open-bsl-format-header-tests-1008.patch` and `/tmp/open-bsl-remaining-test-expectations-1008.patch`; the four stale version-header lines and the previously unsupported-function expectation were updated accordingly. The full rerun is recorded separately before closing the workspace gate.
- [x] Regenerate the API reference after moving the existing function from component registration to the VM-aware builtin path; generator comparison passes.
- [x] Complete fmt, workspace Clippy/tests, warning-free rustdoc, facade consumer build and unchanged core/VM normal dependency checks; investigate any existing regression failure before editing its test.
- [x] Run the release hot-code screen against HEAD: both dispatch functions differ across the accumulated working tree; record scope and cleanup, and make no performance claim without alternating A/B.
- [x] Archive only after implementation, review and required gates succeed.

Verification on 2026-10-08: workspace tests passed (3313 passed, 0 failed, 1 ignored); fmt, all-target Clippy, warning-free rustdoc and facade consumer build passed. Core/VM normal dependency trees match the previous baseline; checked-in API reference and open-question registry checks passed. Logs: `/tmp/open-bsl-workspace-tests-approved-1008.log`, `/tmp/open-bsl-workspace-clippy-approved-1008.log`, `/tmp/open-bsl-workspace-doc-approved-1008.log`, `/tmp/open-bsl-consumer-approved-1008.log`. Existing test updates were explicitly approved by the user before application.
