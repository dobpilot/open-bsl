## Tasks
- [x] Verify that compilation context is server-only and kind is runtime data.
- [x] Validate additive API before implementation.
- [x] Add failing API/reuse/isolation checks in an external unit-test module.
- [x] Implement Clone and consuming with_kind using existing Engine cloning.
- [x] Verify all kinds, retained opening checks, state and parameter isolation.
- [x] Use the API in the original background control with separate connections.
- [x] Run required integration gates and archive after prerequisites pass.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
