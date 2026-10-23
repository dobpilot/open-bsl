## ADDED Requirements

### Requirement: Bounded parallel configuration preparation
Native rooted configuration preparation SHALL perform independent export discovery and selected module analysis with at most two workers, capped by available parallelism. OPEN_BSL_PREPARE_THREADS=1 SHALL select one worker and 2 SHALL request two within that cap; other values SHALL use the default. WebAssembly SHALL prepare sequentially. Thread scheduling SHALL preserve the selected module set, serialized catalog, initialization order, diagnostic types and first-error precedence. Preparation SHALL NOT execute BSL or share a runtime State or database session between workers.

#### Scenario: Compare worker counts
- **WHEN** the same rooted configuration is prepared with one and two workers
- **THEN** its serialized catalog and execution result match
- **AND** temporary full import environments and syntax trees are limited to the active workers

#### Scenario: A later module fails first in wall-clock time
- **WHEN** independent selected modules fail and a later module finishes first
- **THEN** the diagnostic follows the original recipe or discovery order as appropriate
- **AND** explicit-import and deferred-error cycle precedence remain unchanged

#### Scenario: Keep unreachable bodies out of parallel analysis
- **WHEN** the catalog contains a syntactically valid unreachable body with a semantic error
- **THEN** parallel preparation does not resolve that body solely to fill a worker

#### Scenario: Prepare on WebAssembly
- **WHEN** configuration preparation runs on WebAssembly
- **THEN** it uses the sequential path regardless of the worker environment setting

### Requirement: Owned semantic transfer
The semantic crate SHALL provide an opaque owned SendableResolvedProgram through consuming ResolvedProgram::into_sendable and SendableResolvedProgram::into_resolved conversions. Round trips SHALL preserve the entire resolved program, including exact numeric values, links, defaults, lines and deferred errors. Existing IR types and runtime number ownership SHALL remain unchanged. The transfer form SHALL NOT expose mutable numeric payloads or use unsafe Send/Sync implementations.

#### Scenario: Transfer large numeric literals
- **WHEN** a resolved module with both inline and arbitrary precision numbers crosses a worker boundary
- **THEN** conversion back produces an equal resolved module and the same compiled catalog
