## Tasks
- [x] Preserve the original BUH failure and minimal global-capacity reproducer.
- [x] Demonstrate why separate module name namespaces are unsafe for shared values.
- [x] Validate this change strictly before implementation (`openspec validate fit-configuration-method-name-indices --strict`).
- [x] Finalize a chunk-based collection mechanism; retain the native-source lexical survey (34877 method names). The original startup subsequently compiled all 9976 BUH modules successfully.
- [x] Add new capacity and facade tests that fail before implementation, plus link/overflow and ordinary-path controls; existing tests unchanged.
- [x] Implement the guarded configuration retry with explicit genuine-overflow failure.
- [x] Pass five new tests, image verification, textual round-trip, dynamic-prefix execution and the complete optimization corpus.
- [x] Run all gates:3719 passed,six unchanged failures,one ignored; Clippy/rustdoc/fmt/strict passed,consumer compiled,RT/VM dependency trees identical. Sandbox-only socket failures were excluded by a second complete host-capable run.
- [x] Replay the unchanged original BUH with frozen reviewed SDK 0.9.20: all9976 modules compiled in555.03s; startup failed later on field access (exit1,654.32s,peak9408128KiB).
- [x] Archive only after required green checks.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
