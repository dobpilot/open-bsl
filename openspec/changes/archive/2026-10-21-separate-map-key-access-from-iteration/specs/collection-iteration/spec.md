## ADDED Requirements

### Requirement: Map brackets use keys independently of iteration
Ordinary Map brackets SHALL retrieve values by key for supported key types, including Type, Number, UUID and configuration reference values. Missing keys SHALL return Undefined. For Each SHALL retain positional traversal of KeyAndValue pairs through the collection iteration protocol.

#### Scenario: Type key in original session code
- **WHEN** a reference grouping map is indexed by TypeOf of a reference
- **THEN** the stored value for that type is returned or Undefined if absent
- **AND** no numeric index conversion is attempted

#### Scenario: Numeric key and traversal
- **WHEN** a map stores a value under numeric key zero
- **THEN** brackets with zero return that value
- **AND** For Each still visits its KeyAndValue pairs

#### Scenario: Bracket assignment
- **WHEN** a value is assigned through map brackets with a type key
- **THEN** bracket and Get reads observe the updated value
