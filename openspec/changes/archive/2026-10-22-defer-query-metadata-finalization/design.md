# Design

The SDK owns metadata normalization and resource interpretation. It exposes a
separate RuntimeMetadataFacts type containing the normalized facts needed by
ORM projection. It is not a MetadataSnapshot and cannot prepare queries or
claim a query fingerprint. A complete MetadataSnapshot retains an inexpensive
read-only runtime_facts view for existing facade projection entry points.

DatabaseConfigurationSource maintains runtime-facts readiness separately from
query-snapshot readiness. Acquiring a new descriptor invalidates derived
readiness. ORM object preparation resolves its dependency closure and projects
runtime facts without building query indices, reports or fingerprints. Explicit
metadata_snapshot and query paths finalize the current acquired resources via
the existing strict snapshot pipeline. Their cached snapshot is reused until
the acquired composition changes. No old snapshot is mutated or relabelled.

Normalized named types, reference dependencies, separators, extension facts,
predefined values and task target facts remain SDK responsibilities. No raw
metadata semantics are reimplemented in the host. Missing runtime dependencies
must still be acquired or fail; no incomplete runtime view is published.

Private metadata collection indices preserve catalog order and subsystem
parent scope. Explicit common-attribute membership remains separately ordered.
These indices change implementation cost only and are covered by compatibility
tests; no new language semantics are introduced.

Evidence is in docs/research/performance/session-initialization-hotspots-2026-10-22.md.
Measure alternating immutable baseline/candidate runs with one CLI at a time;
record startup result, phase times, instructions and RAM. Do not infer a speedup
from asymptotics or sum nested CPU/timing shares.
