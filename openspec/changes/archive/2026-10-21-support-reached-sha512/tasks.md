# Tasks

- [x] Validate OpenSpec before implementation.
- [x] Retain complete native SHA512 controls and reproduce the current failure.
- [x] Implement SHA512 and replay native controls.
- [x] Repeat original demo startup.
- [x] Run required checks and archive after validation.

Native 1C 8.3.27.2342 control has six complete lines without an error. Interpreter output matches byte for byte. Empty digest, abc, incremental input and repeated snapshots are covered. Existing crypto tests are unchanged. Workspace Clippy passed.

Original demo passed the SHA512 constructor and reached an unsupported Add input type. Initialization is not yet complete.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
