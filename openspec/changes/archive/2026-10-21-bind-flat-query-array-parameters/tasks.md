## Tasks
- [x] Measure live array identity, a matching reference list, empty and nonmatching lists on native 1C.
- [x] Add failing scalar/reference array transport and snapshot tests.
- [x] Convert flat arrays using the existing SDK list carrier and scalar codecs.
- [x] Verify nested/cyclic input fails explicitly and run required checks.
- [x] Replay original demo with the corrected root metadata identity.

Final verification (2026-10-21): Native evidence is retained in `docs/research/fixtures/query-array-parameter-native-1015/`. Both tests in `crates/bsl-config-db/src/data_access_array_tests.rs` passed, including nested/cyclic rejection and immutable prior snapshots. The final workspace log in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/` records 3999 passed, zero failed, one ignored, including the optimizing-pass corpus check; all eight gates passed. Original DEMO initialization passed for thin-client, web-client and background Session initialization in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Protected export stubs remain the approved default; no protected-body equivalence is claimed.
