## Tasks
- [x] Retain native System settings and actual acquired UNF schema evidence.
- [x] Specify the narrow adapter extension and validate before implementation.
- [x] Add a failing store-selection check and share the existing settings read path.
- [x] Verify Common/System scope selection, decoding and explicit failures against SDK 0.9.17.
- [x] Replay original UNF read-only and record its terminal result (ThinClient exit0, unf-system-settings-1015-startup.log).
- [x] Run required checks and archive only after the gates pass.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
