## Tasks
- [x] Read the exact installed syntax-help page and measure live mutations, identity, initial state and key spelling on 1C.
- [x] Add failing focused property tests.
- [x] Replace duplicate storage with the existing runtime Structure and expose both property aliases.
- [x] Apply the separately approved one-line prior expectation correction, if authorized.
- [x] Regenerate the API reference and run required checks.
- [x] Replay original demo and record the next terminal result.

Audit 2026-10-21: the user-approved aggregate patch
`docs/plans/patches/session-existing-test-expectations-proposal-1016.patch`
contains the exact prior spelling correction in `crates/bsl-orm/src/tests.rs`;
the current source expects `зНаЧеНиЕ`. All four live-structure property tests
and the existing adapter suite passed in
`docs/research/fixtures/on-demand-session-preparation-1020/checks-approved-final/workspace-tests.log`.
The latest VM cache change still awaits its separately proposed existing-test
adaptation and current global gates, so this audit does not close those gates.

Final verification (2026-10-21): the historical pending-gate note above is superseded. All four focused tests and generated API-reference freshness passed in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/workspace-tests.log`; all eight gates passed (3999 workspace tests passed, zero failed, one ignored). Original DEMO startup passed in all three session kinds in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Native behavior evidence remains in `docs/research/fixtures/query-parameters-property-native-1015/`.
