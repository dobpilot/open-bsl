# Tasks
- [x] Retain completed native control evidence and inspect existing reference/context paths.
- [x] Validate this change with openspec validate --strict before implementation.
- [x] Project acquired chart predefined names and register chart reference types.
- [x] Resolve chart manager properties and typed query results through the current session.
- [x] Add focused tests for identity, aliases, read-only properties, context scope and explicit local variable precedence.
- [x] Add a focused check for module variable precedence.
- [x] Replay original buh initialization and record its next result: chart predefined names resolve; next failure is own receiver EmptyRef.
- [x] Run formatting, Clippy, workspace tests and rustdoc; regenerate the API reference if affected.
- [x] Archive after verification.
- [x] Register and test the measured chart EmptyRef receiver method.

Final verification (2026-10-21): the exact focused metadata lookup, complete native compatibility-dot and chart predefined-reference/context/empty-reference tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight gates pass (4006/0/1), including generated API freshness, consumer, docs and unchanged core dependency trees. Original DEMO/BUH initialization receipts are retained in the nine-case SDK0.9.35 matrix; the SDK0.9.36 matrix is a separate pending repeat, not claimed completed here. Historical pending test adaptations have been reconciled in the current green gate.
