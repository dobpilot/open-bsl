# Tasks

- [x] Validate strictly before implementation.
- [x] Retain native simple-argument replacement controls.
- [x] Implement shared conversion and add a native conformance fixture.
- [x] Replay replacement and search controls in both execution modes.
- [x] Repeat original demo and required workspace checks.

All ten native replacement lines match ordinary and optimized execution. The nineteen native search lines also replay after shared conversion. Existing tests are unchanged.

Original demo passed replacement and reached an unrelated worker-image buffer allocation abort. Workspace tests before the internal buffer fix passed: 3582 passed, zero failed, one ignored.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
