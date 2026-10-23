## Tasks
- [x] Retain native constructor/type/arity controls and vendor pages.
- [x] Validate strictly before code.
- [x] Reproduce the missing declaration with external tests.
- [x] Add aliases, type and explicit unsupported-execution boundary using existing ORM tables.
- [x] Regenerate the API reference and run focused and workspace checks (3706 passed, 3 existing expectation failures, 1 ignored; Clippy, rustdoc, fmt and strict passed).
- [x] Replay original BUH and retain the terminal result (exit1 after 250.39s; constructor compilation passed, next failure is catalog tabular section ПричиныОтказа).
- [x] Archive only after required green checks.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
