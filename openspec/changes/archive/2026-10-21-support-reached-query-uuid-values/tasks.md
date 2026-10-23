## Tasks
- [x] Add focused adapter parameter/result tests and implementation using the existing UUID codec.
- [x] Verify typed SDK and BSL query results against the own native fixture.
- [x] Replay original demo and UNF through read-only connections and record terminal results.
- [x] Integrate the prepared SDK/adapter and run required workspace checks.

Final reconciliation (2026-10-21): real read-only BSL/SDK PostgreSQL UNION ALL and UUID projection are retained in `docs/research/fixtures/unused-query-parameters-native-1015/bsl-adapter.log` (matching UUID gives two rows, other/nil give zero, canonical typed UUID is preserved). The three focused `data_access_uuid_tests.rs` tests pass in the new SDK0.9.36 `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`, including strict mismatched-cell rejection and reference identity. All eight root gates pass (4006/0/1); old original DEMO/UNF read-only startups are retained in the nine-case SDK0.9.35 matrix. The refreshed SDK0.9.36 matrix is prepared separately and not claimed completed here.
