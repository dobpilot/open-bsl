# Tasks
- [x] Retain native alias/omission and catchable unknown-method evidence.
- [x] Add a failing facade regression for the reached original call shape.
- [x] Preserve open-call defaults without relaxing known closed builtins.
- [x] Run focused tests, compiler optimization conformance, workspace gates and strict validation.
- [x] Replay original BUH and retain the next result.
- [x] Archive after all prerequisites pass.

Validation: four new tests pass; full workspace3677passed/2existing failures/1ignored/0filtered. Compiler optimization conformance passes. Workspace all-target Clippy, rustdoc with warnings denied, fmt and strict OpenSpec107/107 pass. Existing UUID rights and query parameter-key expectation patches remain unapproved and unapplied; archiving is deferred. Original BUH passes the reached omitted call, then fails on SDK19 document attribute Состояние; native CF/XML SDK20 request is prepared.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
