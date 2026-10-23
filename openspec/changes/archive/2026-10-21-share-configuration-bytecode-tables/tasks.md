# Tasks

- [x] Preserve the allocator failure and identify repeated shared tables in the public writer.
- [x] Validate this change strictly before implementation.
- [x] Add a failing hand-built compact-size and shared-identity round-trip test.
- [x] Implement version45 shared-table encoding and parsing without changing VM operands.
- [x] Cover mixed tables, standalone/no-base references and invalid reference directives.
- [x] Prepare exact existing format-header test corrections for user approval; do not apply without approval.
- [x] Add and implement shared ordered import suffixes after strict validation; cover bounds and differing prefixes.
- [x] Run focused representation/compiler round-trip checks, optimization corpus and required workspace gates; distinguish inherited failures. Eight new representation tests and seven compiler round-trip tests pass; full workspace: 3729 passed, nine failed, one ignored. Six pre-existing failures and three format-header failures remain; proposed existing-test corrections are not applied without approval. Formatting, strict validation, Clippy, rustdoc, consumer and dependency-tree checks pass. Evidence: `docs/research/fixtures/buh-startup-job-runtime-allocation-1016/shared-imports-fix/gates/`.
- [x] Rerun immutable original BUH startup under unchanged read-only isolation and memory ceiling. The 989211225-byte worker image is written successfully under the 12 GiB address-space ceiling; startup then fails compiling the unsupported AccessToken constructor. All 28 source/binary hashes match. This is not a successful startup. Evidence: `docs/research/fixtures/buh-startup-job-runtime-allocation-1016/shared-imports-fix/original-startup/result.json`.
- [x] Archive only after required green validation.

- [x] Preserve selected-stage allocation stacks after successful original BUH startup; distinguish diagnostic/baseline sizes and verify all34watched hashes.
- [x] Release the finished temporary image and trim immutable worker String capacity without changing verifier, format or eager-cache policy.
- [x] Run existing job/regression checks and repeat startup plus selected parameter under the unchanged12GiB ceiling; retain before/after capacity and allocator behavior, without claiming an unmeasured performance gain. Workspace3737passed/9unchangedfailed/1ignored. Buffer capacity shrinks to989212994bytes in place; original startup succeeds, selected read still OOM. Evidence: docs/research/fixtures/buh-selected-parameter-allocation-1016/worker-trim-run/.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
