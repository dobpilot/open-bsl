# Tasks
- [x] Confirm reached inline bytes and existing SDK decoder recognition.
- [x] Retain independent native storage control and bounded framing evidence.
- [x] Validate strictly before implementation.
- [x] Obtain the shared SDK text decoder entry point (isolated commit 05a24e5; main integration pending).
- [x] Expose and test the existing runtime decoder through the closed facade.
- [x] Reuse existing snapshot construction for decoded storage contents.
- [x] Classify acquired storage fields and materialize only supported inline results.
- [x] Verify malformed, oversized, unclassified binary and external-reference failures.
- [x] Replay original UNF and run required checks before archive.

Boundary audit (2026-10-21): `docs/research/fixtures/deferred-prepared-module-caches-1021/storage-path-audit-sdk36.json` maps acquired declaration classification, the exact shared SDK decode call and error propagation, runtime parsing and graph rejection. SDK `tests/stored_value_text.rs` covers both size limits, malformed framing/UTF8 and STORHDR; native fixtures in `tests/fixtures/stored-value-text-native-1013/` explicitly establish rejection of a distinct unsupported 0201 envelope, not native positive support for that format. Synthetic 0101 fixtures and the separately recorded reached UNF receipt have different evidentiary scopes. Unclassified Binary columns never enter the storage decode branch. This is shared-helper testing plus source inspection, not a claim of direct SQL tests for every negative input. The refreshed original UNF receipt remains pending in the SDK0.9.36 sequential matrix; the final task stays open.

Final integration verification (2026-10-21): immutable SDK0.9.36 DEMO thin/web/background initialization and UNF ThinClient initialization pass in `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`; UNF ThinClient exit0/canary passed at258.21s. The remaining matrix is still running, not claimed complete. All eight global checks pass (4006/0/1) in `deferred-prepared-module-caches-1021/sdk36-final-checks/`. Catalog lookup/selection additionally pass all eight actual PostgreSQL controls in `catalog-manager-sql-1021/`, including full24 String and16 typed native lines. Logical-result tests cover exact projections and invalid recipes. Storage boundary proof is explicitly composed from shared SDK decoder tests, runtime/facade snapshot tests and source error propagation, as mapped in `storage-path-audit-sdk36.json`; it is not a direct SQL test of every error input. The native0201 envelope remains unsupported and its negative evidence is retained. Unknown/unresolved storage types fail graph capture; no fabricated values or broader storage grammar are claimed. API freshness, facade closure and unchanged core dependencies pass.
