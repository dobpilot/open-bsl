## ADDED Requirements

### Requirement: Typed empty document reference
A document manager SHALL expose zero-argument ПустаяСсылка / EmptyRef through its finite receiver method context. It SHALL return an immutable empty reference of its owning document, not a catalog reference or Undefined. Type names, presentation, UUID, emptiness and equality SHALL match the preserved native measurement.

#### Scenario: Own manager empty reference
- **WHEN** an original document manager module calls bare EmptyRef
- **THEN** it obtains an own document reference with nil UUID and unfilled state
- **AND** repeated own empty references compare equal without reading a document record

#### Scenario: Distinct reference families
- **WHEN** empty document and catalog references are compared or their qualified types are resolved
- **THEN** their concrete types remain distinct
- **AND** Russian and English document-qualified aliases resolve to the same document type

### Requirement: Document reference transfer retains its type
The existing immutable value graph SHALL reconstruct an acquired document reference using its declared document type and UUID without transferring a live session or converting it to a catalog reference. Catalog-specific public decomposition helpers SHALL reject document references.

#### Scenario: Reconstructed empty reference
- **WHEN** an empty document reference is stored or transferred through the existing graph
- **THEN** it retains its owning document type, nil UUID and equality
- **AND** catalog-only decomposition does not classify it as a catalog reference
