## Tasks
- [x] Measure metadata identity, method aliases, primitive type and invalid argument on native 1C.
- [x] Add failing facade-level tests for existing reference types and primitive/invalid cases.
- [x] Implement lookup through existing immutable type declarations.
- [x] Regenerate API reference and run required checks.
- [x] Replay original demo and record its terminal result.

Final verification (2026-10-21): the exact focused metadata lookup, complete native compatibility-dot and chart predefined-reference/context/empty-reference tests pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. All eight gates pass (4006/0/1), including generated API freshness, consumer, docs and unchanged core dependency trees. Original DEMO/BUH initialization receipts are retained in the nine-case SDK0.9.35 matrix; the SDK0.9.36 matrix is a separate pending repeat, not claimed completed here. Historical pending test adaptations have been reconciled in the current green gate.
