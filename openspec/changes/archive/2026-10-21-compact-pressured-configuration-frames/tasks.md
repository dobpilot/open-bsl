## Tasks
- [x] Retain original BUH failure and exact register-pressure diagnosis with minimal boundaries.
- [x] Retain native executable arithmetic42, ByRef42/42, call-order46/1/2/LR and shapes42/1/2 controls; retain separate initial ByRef/binary discrepancy.
- [x] Validate strictly before code.
- [x] Add failing external configuration tests, interner/shape checks and guarded-path controls (three controls passed, two capacity tests failed before implementation).
- [x] Implement one-time retry through existing slot allocation (five new tests pass).
- [x] Pass focused tests, image verification and the complete optimization corpus; existing standalone260-depth test also passes.
- [x] Run fmt, Clippy, workspace tests, rustdoc, dependency trees and strict OpenSpec (3714 passed, six known failures, one ignored; other gates passed).
- [x] Replay unchanged original BUH and retain its terminal result (exit1 after464.24s; prior register failure passed, next is TooManyNames in report object method).
- [x] Archive only after required green checks.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
