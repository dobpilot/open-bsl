# configuration-value-transfer Specification

## Purpose
Preserve UUID and explicitly declared immutable configuration value identities
through the existing value graph, with bounded storage and isolated host state.

## Requirements

### Requirement: UUID graph leaves
The existing value graph SHALL capture and restore native UUID values without
changing their canonical sixteen-byte identity.

#### Scenario: UUID storage
- **WHEN** ValueStorage captures a UUID and returns its value
- **THEN** the restored UUID compares equal to the original

### Requirement: Explicit immutable configuration value transfer
A host-declared immutable value SHALL be transferable only when its registered
type explicitly supplies a trusted materializer and the value supplies an
immutable snapshot. The graph SHALL carry value identity data only, without
connections, principals or Session containers.

#### Scenario: Catalog reference storage
- **WHEN** ValueStorage captures a supported catalog reference
- **THEN** the returned reference retains its exact catalog type and UUID
- **AND** an empty reference remains empty and unfilled

#### Scenario: Receiving configuration mismatch
- **WHEN** the receiving registry has no matching package and type identity
- **THEN** restoration fails instead of binding by a coincident name

#### Scenario: No declared codec
- **WHEN** capture receives a mutable object or component without a declared immutable codec
- **THEN** it retains the existing unsupported-value failure

### Requirement: Transfer accounting and isolation
The existing graph budget SHALL cover immutable value type identifiers, payload
and auxiliary storage before copying. Capturing and dry measurement SHALL agree.
Worker restoration SHALL use its own State and retain only immutable engine declarations.

#### Scenario: Bounded snapshot
- **WHEN** a supported immutable payload exceeds the available graph budget
- **THEN** capture fails before allocating its copied payload

#### Scenario: Worker reference parameter
- **WHEN** a worker receives a catalog reference parameter from its parent
- **THEN** it restores an equal reference using immutable catalog declarations
- **AND** it does not receive the parent's connection or host-data container
