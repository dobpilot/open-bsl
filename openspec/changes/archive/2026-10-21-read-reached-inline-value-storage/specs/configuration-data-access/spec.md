## ADDED Requirements

### Requirement: Acquired inline ValueStorage results
The adapter SHALL materialize a supported inline ValueStorage cell only when the acquired field declaration identifies ValueStorage. The result SHALL be the existing BSL ValueStorage object whose Get returns the decoded value with existing detached snapshot semantics. Reading SHALL retain applicable rights and mandatory data-separation checks.

#### Scenario: Original extension settings storage
- **WHEN** the original session reads a declared ValueStorage field with supported inline content
- **THEN** the result is a ValueStorage object and Get returns its decoded content

#### Scenario: Unclassified binary column
- **WHEN** an unrelated Binary column happens to share a storage-like prefix
- **THEN** it is not silently treated as ValueStorage

#### Scenario: Unsupported or malformed representation
- **WHEN** content is malformed, oversized, externally referenced, or contains an unsupported serialized value
- **THEN** an explicit error is returned without an empty successful replacement

### Requirement: Closed facade internal-value decoder
The facade SHALL expose a pure Rust entry point to the existing internal-value decoder using facade-owned Value and RuntimeShapes names. It SHALL retain the existing BSL decoder's accepted forms and error behavior without a separate parser.

#### Scenario: Facade-only decoding
- **WHEN** a consumer decodes a measured internal serialization using only facade names
- **THEN** it receives the same value or error as the existing runtime decoder
