# Tasks
- [x] Retain original BUH failure/source and native constructor/type/arity controls.
- [x] Add red facade regressions for dormant aliases, type lookup, arity and explicit execution failure.
- [x] Append the constructor/type using existing component tables only.
- [x] Obtain approval before applying the existing density-test expectation update.
- [x] Regenerate the API reference and record focused and technical gate results, including the pending density-test approval.
- [x] Replay original BUH and retain its actual result (exit1 at the next missing chart-label member; constructor execution is not proven).
- [x] Complete the full regression gate and archive after prerequisites pass.

Final verification (2026-10-21): all four chart declaration tests and both planner/facade immutable-map tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight checks pass (4006/0/1), including API-reference freshness, consumer and warnings-denied rustdoc. The prior chart density expectation 1..=3 to 1..=4 was corrected in the user-approved aggregate `docs/plans/patches/session-existing-test-expectations-proposal-1016.patch`. Original DEMO/BUH initialization passed in the SDK0.9.35 nine-case matrix; SDK0.9.36 refresh is a separate live sequential run. No working chart object model or protected-body execution is claimed.
