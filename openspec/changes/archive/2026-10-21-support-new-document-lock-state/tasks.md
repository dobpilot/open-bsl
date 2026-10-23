## Tasks
- [x] Retain native selected/body/alias/arity behavior and full vendor page.
- [x] Validate before implementation.
- [x] Reproduce the BUH receiver binding failure in a focused test.
- [x] Add the descriptor and selected-new-object result through existing machinery.
- [x] Verify native selected/body results, arity, local precedence and existing document checks.
- [x] Regenerate API and run focused document tests, all-target Clippy, rustdoc and formatting.
- [x] Repeat original BUH after the active UNF background control.
- [x] Complete integration gates and archive after prerequisites pass.

Final verification (2026-10-21): all eight SDK 0.9.36 workspace gates passed, including API-reference freshness and facade consumer (`docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/`). Original BUH thin-client initialization returned exit 0 in 292.09 seconds (`docs/research/fixtures/on-demand-session-matrix-sdk36-1021/buh-thin-client/cli-buh-on-demand-preparation-1020-result.json`). Default protected export stubs remain in effect; this is not a protected-body parity claim.
