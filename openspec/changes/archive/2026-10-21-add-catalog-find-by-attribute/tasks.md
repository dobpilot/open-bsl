# Tasks

- [x] 1. Inspect original source and full help; preserve native oracle and validate strictly before implementation.
- [x] 2. Implement measured attribute lookup via the current principal-bound adapter and receiver table.
- [x] 3. Verify native references, no match, arguments, aliases, adapter failures and isolation without modifying existing expectations.
- [x] 4. Regenerate API reference and repeat UNF startup.
- [x] 5. Run workspace gates, consumer, dependency checks and strict validation; archive only after green results.

## Remaining verification

- The live minimal UNF/buh session has not produced an SQL result. The first type-inspection defect is fixed; the subsequent field-read denial is preserved. Recheck with an eligible principal whose acquired roles grant Read, then initialize separator parameters through the original session path.
- Optional owner remains explicitly unsupported until complete owner metadata is acquired from SDK.
- Hierarchical parent behavior is measured in a separate six-row native oracle.
- Number, Boolean, Date and own Reference behavior is measured in a separate sixteen-row native oracle.
- Catalog GetTemplate linking is covered by the separate validated change and awaits the requested approval to correct one preexisting negative assertion.
- Do not mark tasks 2–5 complete from mock-adapter tests or a successful workspace run alone.

Verification update (2026-10-21): tasks 2 and 3 are now verified by actual BSL→DatabaseDataAccess→SDK→PostgreSQL execution on a disposable cluster in `docs/research/fixtures/catalog-manager-sql-1021/`. All eight tests pass on the SDK 0.9.36 candidate, including the complete unchanged 24-line String oracle, complete unchanged 16-line typed oracle, measured parent-profile composition and denied-principal errors. Existing Session tests retain adapter-failure/isolation/aliases/argument coverage. The original historical minimal UNF attempt above remains a denied pre-SQL attempt, not successful SQL evidence; the new own-cluster controls are separately scoped. Unknown or nonempty owner metadata remains explicitly unsupported; only acquired complete empty ownership now ignores the fourth argument as measured. Tasks 4 and 5 remain open pending final integration checks and refreshed startup/gates; no final SDK acceptance is claimed.

Final integration verification (2026-10-21): immutable SDK0.9.36 DEMO thin/web/background initialization and UNF ThinClient initialization pass in `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`; UNF ThinClient exit0/canary passed at258.21s. The remaining matrix is still running, not claimed complete. All eight global checks pass (4006/0/1) in `deferred-prepared-module-caches-1021/sdk36-final-checks/`. Catalog lookup/selection additionally pass all eight actual PostgreSQL controls in `catalog-manager-sql-1021/`, including full24 String and16 typed native lines. Logical-result tests cover exact projections and invalid recipes. Storage boundary proof is explicitly composed from shared SDK decoder tests, runtime/facade snapshot tests and source error propagation, as mapped in `storage-path-audit-sdk36.json`; it is not a direct SQL test of every error input. The native0201 envelope remains unsupported and its negative evidence is retained. Unknown/unresolved storage types fail graph capture; no fabricated values or broader storage grammar are claimed. API freshness, facade closure and unchanged core dependencies pass.
