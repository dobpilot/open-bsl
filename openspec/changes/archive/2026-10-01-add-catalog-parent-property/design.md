# Design

## Native evidence
Native syntax helper 8.3.27.2342, Russian document platform-syntax-help:syntax-context:objects/catalog125/catalog126/object130/properties/Parent1611.html states Parent is a reference to the same catalog and root-level items have an empty reference.

The paired native file-infobase probes measure both Hierarchical=false and true: Parent reads in the module body succeed with the own empty reference; writes in the body fail before selection. The fresh selected object initially exposes that same own reference. Nonhierarchical objects reject even an empty-reference write. Hierarchical objects accept an own filled reference; Undefined, Null, numeric, boolean and string values become the own empty reference. Both full oracles include thirteen assignments. A UUID object, array, structure, live new object, and empty or filled foreign catalog references all become the own empty reference in a hierarchical catalog; the nonhierarchical catalog rejects every assignment. The reference IsEmpty method returns false only for the retained own filled reference in this matrix.

SDK derives catalog_hierarchical: Option<bool> from a measured owner field, with 296 UUID-matched native XML catalogs (219 false, 77 true). The host preserves None rather than assuming a hierarchy setting.

## Implementation
Store Parent per concrete object, initialized through the existing catalog reference constructor. Reads use this local value without database access. Writes require a selected object, then a known hierarchy setting. A known false setting is readonly; an unknown setting gives an explicit metadata-unavailable error. An accepted reference must retain the owning catalog type. This change does not implement saving or hierarchy constraints checked when records are persisted.

The reference IsEmpty method observes UUID emptiness through the same existing reference representation. No second reference representation, graph codec, resolver exception or VM instruction is introduced.
