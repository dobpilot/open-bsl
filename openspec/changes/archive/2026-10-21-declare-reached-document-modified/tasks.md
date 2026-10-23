## Tasks
- [x] Retain full vendor page and native signature/alias/bare/arity evidence.
- [x] Validate the narrow change before implementation.
- [x] Reproduce receiver binding failure and append the measured descriptor.
- [x] Verify scoped calls, unsupported execution, arity and local-function precedence.
- [x] Regenerate API and run focused tests, all-target Clippy and rustdoc.
- [x] Replay original BUH and retain its terminal result (next failure: IsLocked).
- [x] Complete the full integration gate and archive after prerequisites pass.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
