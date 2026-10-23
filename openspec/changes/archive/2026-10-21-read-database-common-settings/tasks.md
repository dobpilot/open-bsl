## Tasks
- [x] Connect context-aware common-settings loads to SDK SQL and decoding.
- [x] Verify own native scalar, missing, storage and separated values through BSL.
- [x] Test missing/duplicate/invalid rows and unsupported operations.
- [x] Replay original demo with the adapter.
- [x] Run focused tests and checks; integrate the prepared SDK before final workspace gates.

Audit 2026-10-21: the first item is implemented in
`crates/bsl-config-db/src/data_access/settings.rs::load_with_context`: it validates
the Session principal, checks namespace rights, obtains separation conditions,
compiles SDK SQL, reads the retained connection and decodes scalar or value-storage
payloads through existing materialization. Six local row/scope/rights tests pass in
`docs/research/fixtures/on-demand-session-matrix-fixed-dependency-1021/config-db-tests.log`.
Those tests do not establish complete native BSL/database replay, unsupported-operation
test coverage or current global gates; the remaining items stay open.

Final reconciliation (2026-10-21): actual read-only BSL/SessionFactory/DatabaseDataAccess/SDK controls are retained in `docs/research/fixtures/user-settings-native-1015/adapter/` (scalar, missing, storage, scoped 11/12 reads, other-user/store/write refusals); numeric native-condition replays are in `common-settings-numeric-native-1015/`. Later `conditional-false-use-sdk29-adapter-1019/` verifies all four String/Numeric Independent/Shared profiles, global-condition false with use=false and no unnecessary parameter initialization. Earlier Shared failures in conditional-separation-adapter-1016 are historical red evidence, not used as acceptance. Duplicate/invalid-row and rights tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`; all eight root checks pass (4006/0/1). Original DEMO startup with the adapter is retained in production-cli and the current SDK0.9.36 three-kind DEMO matrix receipts. SQL Server live coverage is not claimed.
