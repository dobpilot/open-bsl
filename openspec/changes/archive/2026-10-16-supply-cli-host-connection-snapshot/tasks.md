# Tasks

- [x] Validate this change strictly before implementation.
- [x] Supply the actual CLI connection snapshot after acquired-role authorization, with explicit host identity.
- [x] Test authorized and denied access, complete host membership and unchanged unavailable-snapshot behavior.
- [x] Replay original DEMO startup and selected parameter initialization with retained evidence.
- [x] Run formatting, Clippy, workspace tests, rustdoc and consumer/dependency checks; archive only after successful required checks.

## Verification so far

The complete host-connection workspace run finished with 3760 passed, one inherited SDK SaveUserData GUID failure and one ignored test. Formatting, strict validation, Clippy, rustdoc, consumer and unchanged RT/VM normal dependency checks pass. Original DEMO ThinClient gets an authorized complete singleton snapshot and passes the former barrier; startup then fails on the missing IntegratedSystems exchange plan. Native Config/DBNames inspection identifies a separate existing projection typo, retained in docs/research/fixtures/demo-exchange-plan-class-1016. Startup and selected initialization remain unproven, so the replay and final gates/archive tasks remain open.

DEMO ThinClient, WebClient and parent/BackgroundJob now pass on the measured exchange-plan implementation; selected initialization passes from unset to set. All frozen hashes match. Actual configured CLI dispatch also passes in the prepared SDK20/settings/journal integration. ROOT after the final reference implementation:3769passed/1inherited SDK GUID failure/1ignored. No archive is performed while the dependency result and required deployment integration remain incomplete.

Final authorized integration verification (2026-10-16): ROOT3794passed,0failed,1ignored; SDK1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, consumer build and unchanged RT/VM dependency trees pass. Actual production CLI initializes DEMO successfully. The frozen review matrix covers all nine original session contexts plus selected initialization in all three databases. Evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/` and `docs/plans/configured-session-startup-matrix.json`. Earlier failed/pending outcomes above are historical.
