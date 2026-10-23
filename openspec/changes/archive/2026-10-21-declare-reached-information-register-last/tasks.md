## Tasks
- [x] Retain the vendor signature and native own-manager/qualified arity control.
- [x] Add a failing receiver-context regression, declare GetLast and preserve existing exports.
- [x] Verify explicit execution refusal, scope and arity; regenerate API and run checks.
- [x] Replay original BUH and retain its actual terminal result.

Final verification (2026-10-21): declaration, finite arity, own-context scope, alias, existing export/variable precedence and explicit unavailable execution tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight gates pass (4006/0/1), including generated API freshness and facade consumer. The old accumulation-template absence expectation was adapted in the user-approved aggregate `docs/plans/patches/session-existing-test-expectations-proposal-1016.patch`; no new approval is claimed here. Retained original BUH attempts already passed the original missing declarations, and the SDK0.9.35 nine-case matrix completed initialization. SDK0.9.36 refresh is currently running separately. GetLast/actual named-template data retrieval remains explicitly unsupported as the accepted requirements specify.
