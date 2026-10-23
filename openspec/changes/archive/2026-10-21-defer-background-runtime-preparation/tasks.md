# Tasks

- [x] 1. Retain the UNF crash stack and identify eager worker-image printing before the session handler.
- [x] 2. Add a failing test showing ordinary State construction initializes the background runtime.
- [x] 3. Defer preparation to capability access while preserving explicit runtime access and failure diagnostics.
- [x] 4. Verify once-only sharing, missing capability, existing background behavior and isolated host contexts.
- [x] 5. Run formatting, Clippy, tests, rustdoc, facade consumer and dependency checks; respect pending approvals for unrelated test adaptation.
- [x] 6. Repeat original read-only UNF with bounded memory and disabled core dumps; retain the actual session result.
- [x] 7. Archive only after verified gates.

Final verification (2026-10-21): all eight checks passed in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/results.json`; the workspace log explicitly records the three background-runtime tests and facade-only signature test passing. Original initialization passed for all nine configuration/session-kind combinations in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`, including bounded UNF. These runs establish initialization only, not background procedure-body execution. No worker serialization performance improvement is claimed.
