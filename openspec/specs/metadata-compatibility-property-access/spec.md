# metadata-compatibility-property-access Specification

## Purpose
Specify measured configuration metadata lookup, member access and reference behavior through the existing interpreter and session context.

## Requirements

### Requirement: Measured compatibility property member access
Metadata.ObjectProperties.CompatibilityMode and its Russian aliases SHALL support case-insensitive member access through the dot as well as the existing index. DontUse and Version8_3_24 SHALL match the retained native values and type display. Existing bare-name and type-name restrictions SHALL remain unchanged.

#### Scenario: Native dot and indexed access agree
- **WHEN** the retained compatibility-property-dot control executes
- **THEN** all eight output lines match the native oracle

#### Scenario: Unknown member still fails
- **WHEN** a program reads an unknown member through the property enumeration
- **THEN** it receives the existing unknown-property error
