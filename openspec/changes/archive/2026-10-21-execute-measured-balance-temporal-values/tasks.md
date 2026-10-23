## Contract and evidence
- [x] Retain native Balance and Boundary object artifacts separately.
- [x] Validate this change strictly before implementation.
- [x] Review and integrate the final SDK 0.9.28 delivery and exact API.

## Implementation
- [x] Add failing tests for measured Boundary construction, readonly properties, aliases and equality.
- [x] Prepare and obtain approval for replacing existing placeholder-constructor expectations.
- [x] Implement the immutable Boundary wrapper and documented public bridge.
- [x] Convert supported temporal query parameters using existing reference/date conversions.
- [x] Reconstruct PointInTime logical results using validated SDK recipes.
- [x] Forward manager Balance temporal arguments through the same query path.

## Verification
- [x] Compare actual adapter manager/query results with all retained temporal controls.
- [x] Preserve Date behavior, rights/RLS/separation and explicit unsupported diagnostics.
- [x] Regenerate and review the BSL API reference.
- [x] Pass formatting, Clippy, workspace tests, rustdoc, facade consumer and unchanged RT/VM dependency checks.

Verification 2026-10-19: Date/unsupported regressions pass in `docs/research/fixtures/buh-intermediate-lifetimes-1019/full-workspace/tests.log`; actual current-SDK ordinary RLS controls pass in `docs/research/fixtures/ordinary-rls-sdk29-adapter-1019/`, and four String/Numeric Independent/Shared condition controls pass in `docs/research/fixtures/conditional-false-use-sdk29-adapter-1019/`. These controls preserve their documented PostgreSQL/synthetic-principal scope. Full workspace remains3932passed/1known-failed/1ignored; the old Boundary constructor expectation is unchanged pending approval.

Audit 2026-10-21: the Boundary expectation was explicitly approved and applied
on 2026-10-20; its exact patch, authorization and passing focused check are
retained in `docs/research/fixtures/boundary-approved-test-update-1020/result.json`.
The current source contains that correction. The later full3996/0/1 gate in
`docs/research/fixtures/on-demand-session-preparation-1020/checks-approved-final/`
also passes it. Current metadata-dependency and VM-cache changes require their
own final global gate; this task reconciliation does not claim that gate passed.
