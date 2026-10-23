# Tasks
- [x] Preserve native string-code and exact/prefix description evidence and inspect original demo call arguments.
- [x] Implement manager descriptors, receiver context, export compatibility and shared principal-bound queries.
- [x] Verify matching, aliases, arity, failure propagation, reference ownership, session principal and instance isolation.
- [x] Verify the database adapter compiles standard-field lookup through existing rights/RLS handling.
- [x] Regenerate API reference and complete required workspace and dependency checks.
- [x] Repeat original compilation and startup; retain actual barriers.
- [x] Archive after required checks and scope verification.

The `catalog-search-gates-1006` run passed all eight gates. Existing expectations were unchanged. Demo compile-only passed the original lookup manager and next reached `ТочкиМаршрута` in the business-process manager; original session execution was incomplete at that historical checkpoint; the final SDK 0.9.36 receipt below supersedes that barrier.

Final review (2026-10-21): SDK 0.9.36 passed all eight workspace gates, including API-reference freshness, consumer closure and unchanged core dependency trees (4006 tests, zero failures, one ignored dump). Original DEMO thin-client and web-client initialization passed on the immutable SDK 0.9.36 release build. Protected exports retain approved default stubs; this evidence does not cover their bodies. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/catalog-search-enumeration-closure-sdk36.json`.
