# Tasks
- [x] Locate the original UNF query and its explicit privilege scope.
- [x] Retain completed native control of access policy and suppression.
- [x] Validate this change strictly before implementation.
- [x] Integrate effective frame mode into the existing adapter policy path.
- [x] Verify ordinary access, privileged access, reset, suppression and session isolation.
- [x] Replay original UNF: field access passes; next refusal is stored-value binary decoding.
- [x] Run required checks and archive after verification.

Final verification (2026-10-21): four `session_initialization_privilege_tests` and `query_effective_privilege.rs` pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. Native inputs/results are retained in `session-entry-privilege-1016/` and `query-effective-privilege/`; own PostgreSQL functional-option source controls additionally retain ordinary denials and privileged reads. Source inspection confirms the ordinary query adapter reads CallContext effective mode, retains restricted preparation and metadata checks, bypasses field/RLS decisions only for effective privilege, and retains READ ONLY SQL. All eight root gates pass (4006/0/1). Immutable SDK0.9.36 DEMO thin/web/background and UNF ThinClient initialization receipts pass; the complete refresh matrix remains running separately. WebClient automatic entry remains explicitly unmeasured under `SESSION.INIT.WEB_PRIVILEGE`, with source/registry/measure line retained; passing open-bsl WebClient startup is not a platform measurement. No new role grants, thread-local flags or native WebClient parity are claimed.
