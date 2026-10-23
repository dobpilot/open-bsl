# Tasks
- [x] Measure zero-argument aliases, constructor failures, defaults and selected assignment semantics.
- [x] Measure remaining conversions needed by the implemented setters.
- [x] Add the component object, descriptors and reused fixed-map construction through the facade.
- [x] Compare independent execution with native evidence and verify per-instance isolation.
- [x] Regenerate the API reference and pass formatting, Clippy, workspace tests, rustdoc, consumer and unchanged dependency checks.
- [x] Repeat original demo compilation and startup; preserve actual results.
- [x] Archive only after the required checks pass.

Verification: `planner-item-gates-1006` completed all eight gates successfully, including strict validation of48 items. Original demo compile-only passed the planner constructor and reached `СоздатьКлючЗаписи` in the information-register manager. Original startup remains unverified.

Final verification (2026-10-21): all four chart declaration tests and both planner/facade immutable-map tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight checks pass (4006/0/1), including API-reference freshness, consumer and warnings-denied rustdoc. The prior chart density expectation 1..=3 to 1..=4 was corrected in the user-approved aggregate `docs/plans/patches/session-existing-test-expectations-proposal-1016.patch`. Original DEMO/BUH initialization passed in the SDK0.9.35 nine-case matrix; SDK0.9.36 refresh is a separate live sequential run. No working chart object model or protected-body execution is claimed.
