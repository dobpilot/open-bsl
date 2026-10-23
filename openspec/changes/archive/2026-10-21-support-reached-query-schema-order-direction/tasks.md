## Tasks
- [x] Locate both reached members in original BUH and retain native controls and vendor page provenance.
- [x] Reproduce failed lookup and add both members through existing tables.
- [x] Regenerate the API reference and run required checks.
- [x] Replay original BUH and record its terminal result.

Verification update (2026-10-21): the SDK 0.9.36 workspace gate at `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/` passed all eight checks, including API-reference freshness, facade consumer and core dependency comparison (4006 tests, zero failures, one ignored dump). Original UNF thin-client and web-client initialization passed in `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`, with approved default protected export stubs. These receipts do not prove live document number allocation or execution of protected bodies; unresolved tasks remain open.

Final verification (2026-10-21): all eight SDK 0.9.36 workspace gates passed, including API-reference freshness and facade consumer (`docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/`). Original BUH thin-client initialization returned exit 0 in 292.09 seconds (`docs/research/fixtures/on-demand-session-matrix-sdk36-1021/buh-thin-client/cli-buh-on-demand-preparation-1020-result.json`). Default protected export stubs remain in effect; this is not a protected-body parity claim.
