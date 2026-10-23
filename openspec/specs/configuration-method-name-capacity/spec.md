# configuration-method-name-capacity Specification

## Purpose
Keep callable configuration method indices bounded independently of property-only names while preserving existing member resolution.

## Requirements

### Requirement: Configuration method indices fit independently of property-only names
For modules that each compile within the existing name-index limits when compiled independently with configuration-method rules, configuration compilation SHALL succeed when their combined statically required narrow method and component-type names fit the existing operand range even if property-only names make the shared name table exceed that range. All modules and the optional entry SHALL retain one consistent name namespace and compatible structure shapes. Existing successful compilation SHALL preserve name order, first spelling and executable behavior. The serialized format and operand widths SHALL remain unchanged.

#### Scenario: A method follows many property names
- **WHEN** one module introduces 65536 property names and another introduces a new open method name
- **THEN** the configuration compiles and the method call uses the intended name
- **AND** cross-module structures retain their field identities

#### Scenario: Component links and case variants
- **WHEN** an affected catalog uses component method links and different case spellings of shared names
- **THEN** link type and method indices remain valid and each name retains its first source spelling

#### Scenario: Ordinary successful catalogs
- **WHEN** a catalog fits the existing ordinary compilation path
- **THEN** its names, shapes and executable behavior remain unchanged

#### Scenario: A module independently exceeds the current limit
- **WHEN** an individual module cannot compile within its existing name-index limits
- **THEN** configuration compilation retains its contextual name-capacity failure

#### Scenario: Genuine narrow-name overflow
- **WHEN** the number of required narrow names exceeds the existing operand capacity
- **THEN** compilation fails with a contextual name-capacity diagnostic without truncation or a panic

#### Scenario: Image round trip and dynamic prefixes
- **WHEN** an affected compiled catalog is printed and read back, or used as the base for a dynamic entry
- **THEN** existing image verification and common-prefix rules remain satisfied
