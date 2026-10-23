# Tasks
- [x] Identify reached type UUIDs and unique acquired catalog owner.
- [x] Measure UUID and catalog reference scalar serialization in an own native control.
- [x] Validate strictly before implementation.
- [x] Add opt-in resolver to the existing decoder and closed facade.
- [x] Materialize the reached configured reference and UUID without a second parser.
- [x] Test default transit, opt-in nested materialization, malformed input, unknown type and detached snapshots.
- [x] Replay original UNF and run required gates before archive.

Final integration verification (2026-10-21): immutable SDK0.9.36 DEMO thin/web/background initialization and UNF ThinClient initialization pass in `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`; UNF ThinClient exit0/canary passed at258.21s. The remaining matrix is still running, not claimed complete. All eight global checks pass (4006/0/1) in `deferred-prepared-module-caches-1021/sdk36-final-checks/`. Catalog lookup/selection additionally pass all eight actual PostgreSQL controls in `catalog-manager-sql-1021/`, including full24 String and16 typed native lines. Logical-result tests cover exact projections and invalid recipes. Storage boundary proof is explicitly composed from shared SDK decoder tests, runtime/facade snapshot tests and source error propagation, as mapped in `storage-path-audit-sdk36.json`; it is not a direct SQL test of every error input. The native0201 envelope remains unsupported and its negative evidence is retained. Unknown/unresolved storage types fail graph capture; no fabricated values or broader storage grammar are claimed. API freshness, facade closure and unchanged core dependencies pass.
