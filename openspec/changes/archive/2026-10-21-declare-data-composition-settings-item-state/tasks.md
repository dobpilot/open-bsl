# Tasks
- [x] Retain full vendor page and native output for every member, aliases and meta-type.
- [x] Add a failing facade test for the reached enum.
- [x] Extend the existing enum and type tables.
- [x] Regenerate the API reference and verify tests, Clippy, docs and strict validation.
- [x] Replay original BUH initialization and retain its terminal result.

- [x] Complete the full workspace regression gate and archive after prerequisites pass.

Archive preparation2026-10-21: the identical shared Native localized enum type lookup requirement already exists after the group-placement change; its canonical body/scenario are preserved unchanged. Only the duplicate ADDED block is removed from this active delta.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
