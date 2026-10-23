# Add concrete configuration enumeration values

## Why
Original configuration code and document filling use configuration enumeration references. The current placeholder exposes a generic type and cannot supply manager values or empty references. Native 1C probes now establish the concrete reference surface and distinguish configuration script variant from default-language presentation.

## What Changes
Declare concrete enumeration reference types through the existing host value type mechanism. Expose configured members and measured enumeration manager operations, empty references, metadata and presentation. Preserve immutable identities through the existing configuration value graph for document filling and inherited session parameters.

## Impact
The ORM metadata descriptors, configuration acquisition projection, enumeration components, facade integration tests and generated API reference. No alternative evaluator, new core dependency or database persistence operation is introduced.
