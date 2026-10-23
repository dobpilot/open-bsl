## Tasks
- [x] Measure unused UUID/object/retained parameters and a missing used parameter on native 1C.
- [x] Add focused tokenizer-boundary and parameter-conversion tests.
- [x] Filter only the database adapter's supplied parameter list before conversion.
- [x] Verify the own BSL control and replay original demo/UNF.
- [x] Run affected regression tests and checks; integrate into the main adapter.

Verification (2026-10-21): native platform and real database-adapter controls are retained in `docs/research/fixtures/unused-query-parameters-native-1015/` (`platform.oracle`, `bsl-adapter-control.rs`, `bsl-adapter.log`). Token boundaries, retained values and missing used parameters are covered by `crates/bsl-config-db/src/query_parameter_selection_tests.rs`. The production adapter filters before conversion in `execute_query_result`; it does not mutate the query parameter collection. Final workspace verification: 3999 passed, zero failed, one ignored; all eight gates passed in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/`. Original DEMO/UNF initialization passed in all three session kinds in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Protected method bodies remain outside the current goal.
