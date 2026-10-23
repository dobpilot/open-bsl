# Tasks

- [x] Retain the native old-session/new-session history control.
- [x] Validate strictly before implementation.
- [x] Supply the measured initial state in configured CLI resource construction.
- [x] Verify supplied and missing values through public session integration tests.
- [x] Repeat original demo startup and record its terminal outcome.
- [x] Repeat original UNF startup and record its terminal outcome.
- [x] Run required checks, preserving pending existing-test approval.
- [x] Merge and archive after verification.

Final verification (2026-10-21): configured CLI resource construction supplies the initial dynamic status and captures the start Date immediately before opening Session. The external `session_dynamic_configuration_status` and `session_started` tests passed in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/workspace-tests.log`, including native controls, aliases, absent resources, foreground/background isolation and facade timestamp conversion. All eight gates passed (3999/0/1), including API-reference freshness. Original DEMO and UNF initialized successfully for all three session kinds in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Historical pending test adaptations are resolved in that final gate. No native server telemetry or protected-body execution is claimed.
