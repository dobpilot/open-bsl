# configuration-extension-hash Specification

## Purpose
Expose measured binary configuration-extension hashes using the existing facade codec.

## Requirements

### Requirement: Configuration extension hash has the measured binary type
ConfigurationExtension.HashSum and ХешСумма SHALL return BinaryData matching the acquired native hash bytes. The existing Rust metadata representation MAY retain Base64 text. Reading an invalid host-provided Base64 hash SHALL fail explicitly without modifying metadata or the database.

#### Scenario: Native type and encoding
- **WHEN** the retained native control reads the hash and calls Base64String
- **THEN** its type is BinaryData and the encoded result matches the platform oracle
- **AND** ordinary String arguments to Base64String retain their existing behavior

#### Scenario: Invalid host metadata
- **GIVEN** a host provides a hash that is not valid Base64
- **WHEN** the property is read
- **THEN** the call fails rather than exposing an invented hash

### Requirement: Codec access remains inside the facade
Consumers SHALL be able to use the existing Base64 decoder through open-bsl without importing an internal runtime crate or adding a dependency. The runtime decoder SHALL remain the single implementation.

#### Scenario: Facade consumer
- **WHEN** the facade consumer decodes a native acquired hash
- **THEN** it builds and obtains the expected bytes using only open-bsl imports
