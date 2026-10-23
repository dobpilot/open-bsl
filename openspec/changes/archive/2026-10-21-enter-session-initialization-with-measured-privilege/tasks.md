# Tasks

- [x] Preserve and independently inspect native automatic initial/selective handler entry evidence.
- [x] Validate this change with --strict before implementation.
- [x] Add a minimal explicit per-call entry mode to existing handler calls; document public Rust API in Russian.
- [x] Use the mode for automatic initial and selective session parameter initialization.
- [x] Add focused tests for both entries, ordinary generic default, caller restoration after return/error and session isolation; preserve existing tests.
- [x] Register the remaining unmeasured WebClient entry context in source, registry and measurement script.
- [x] Run focused checks, formatting, Clippy, workspace tests, rustdoc, facade consumer and unchanged RT/VM dependency checks; report inherited failures separately.
- [x] Rerun original startup evidence with an immutable updated diagnostic binary.
- [x] Archive only after required checks are green.

Final verification (2026-10-21): four `session_initialization_privilege_tests` and `query_effective_privilege.rs` pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. Native inputs/results are retained in `session-entry-privilege-1016/` and `query-effective-privilege/`; own PostgreSQL functional-option source controls additionally retain ordinary denials and privileged reads. Source inspection confirms the ordinary query adapter reads CallContext effective mode, retains restricted preparation and metadata checks, bypasses field/RLS decisions only for effective privilege, and retains READ ONLY SQL. All eight root gates pass (4006/0/1). Immutable SDK0.9.36 DEMO thin/web/background and UNF ThinClient initialization receipts pass; the complete refresh matrix remains running separately. WebClient automatic entry remains explicitly unmeasured under `SESSION.INIT.WEB_PRIVILEGE`, with source/registry/measure line retained; passing open-bsl WebClient startup is not a platform measurement. No new role grants, thread-local flags or native WebClient parity are claimed.
