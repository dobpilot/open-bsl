## Tasks
- [x] Retain native assignment, invalid inputs, immutable and reference-output evidence.
- [x] Add failing focused tests and implement shared indexed field writes.
- [x] Confirm the original failure operation and replay demo after the fix.
- [x] Run fmt, Clippy, workspace tests, docs and facade closure checks.

Final verification (2026-10-21): `crates/bsl-cli/tests/structure_index_assignment.rs` compares the original operation and all accepted mutation/error/output cases byte-for-byte with the retained native oracle. It and `structure_property_preserves_an_indexed_output_target` pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/workspace-tests.log`; all eight gates pass (3999/0/1). Original DEMO initialization passes in all three session kinds in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. The historical static procedure by-reference discrepancy documented beside the oracle is outside this change and is not claimed fixed.
