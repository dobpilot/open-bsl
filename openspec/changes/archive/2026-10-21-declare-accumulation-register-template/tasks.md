## Tasks
- [x] Retain the original BUH terminal failure and full vendor signature.
- [x] Retain native manager alias, arity and missing-template controls (10 cases, completed=YES).
- [x] Validate strictly before implementation.
- [x] Reproduce the missing declaration in new external tests (1 passed, 3 failed before implementation).
- [x] Add the finite descriptor and preserve export and variable precedence (four new tests passed).
- [x] Obtain approval before changing the prior absence expectation.
- [x] Regenerate the API reference and run focused and workspace checks (3707 passed, 6 known failures (one main-SDK rights mismatch and five pending expectation corrections), 1 ignored; Clippy/doc/fmt/strict passed).
- [x] Replay original BUH and retain the terminal result (exit1 after 259.45s, previous failure passed; next is ТипУзораТабличногоДокумента in report module).
- [x] Archive only after required green checks.

Final verification (2026-10-21): declaration, finite arity, own-context scope, alias, existing export/variable precedence and explicit unavailable execution tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight gates pass (4006/0/1), including generated API freshness and facade consumer. The old accumulation-template absence expectation was adapted in the user-approved aggregate `docs/plans/patches/session-existing-test-expectations-proposal-1016.patch`; no new approval is claimed here. Retained original BUH attempts already passed the original missing declarations, and the SDK0.9.35 nine-case matrix completed initialization. SDK0.9.36 refresh is currently running separately. GetLast/actual named-template data retrieval remains explicitly unsupported as the accepted requirements specify.
