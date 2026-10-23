## Tasks
- [x] Measure current, role and explicit user subjects with and without privileged mode on native 1C.
- [x] Measure unknown and inapplicable rights and retain native role resources.
- [x] Add failing ORM and adapter tests for the reached root requests.
- [x] Add the root request kind and delegate root checks without changing non-root dispatch.
- [x] Evaluate supported root rights through the existing SDK role data.
- [x] Run required checks in separate build directories and replay original configurations.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
