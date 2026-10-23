## ADDED Requirements

### Requirement: Explicit protected module body recovery
The database loader SHALL default to an exported-interface-only source for
image-only protected modules. It SHALL retain exported procedure/function
names, parameter passing modes and default values. Procedures SHALL have
empty bodies and functions SHALL return True. Placeholders SHALL be synchronous,
including measured asynchronous exported declarations; they SHALL NOT claim
the native promise or reference-update semantics. It SHALL omit private methods,
external declarations, module variables and module initialization.
This mode SHALL NOT claim equivalence to the original implementation.

The loader SHALL expose a per-load option to request existing complete body
recovery for both base and extension modules. Complete recovery SHALL reject
unsupported declarations or bodies without falling back to placeholders.
Malformed image tables and unsupported exported signatures SHALL remain errors.
Modules with available source text SHALL remain unchanged in both modes.

#### Scenario: Default exported interface
- **WHEN** an image-only module is loaded without options
- **THEN** only its exported declarations are emitted
- **AND** a procedure performs no work and a function returns True
- **AND** private bodies and initialization do not execute.

#### Scenario: Explicit complete recovery
- **WHEN** complete decompilation is enabled
- **THEN** supported complete modules retain their native observations
- **AND** unsupported modules fail rather than receive placeholder bodies.

#### Scenario: Malformed exported signature
- **WHEN** an exported declaration has an unsupported parameter/default encoding
- **THEN** interface-only loading fails explicitly.

#### Scenario: Text and extension sources
- **WHEN** either a base or extension container has text source
- **THEN** its source is preserved regardless of the decompilation option
- **AND** image-only extension modules use the same option as base modules.

#### Scenario: Asynchronous exported interface
- **WHEN** a supported measured async export flag is present
- **THEN** interface-only loading retains its name and parameters as a synchronous placeholder
- **AND** complete decompilation retains its existing strict recovery rules.

### Requirement: CLI protected body switch
Database script mode SHALL accept `--decompile-protected-modules` to enable
complete body recovery. Omission SHALL select exported-interface-only loading.
The flag SHALL be documented and SHALL require database mode.

#### Scenario: Flag selects complete recovery
- **WHEN** the flag precedes a database script path
- **THEN** factory input loading enables complete decompilation.

#### Scenario: Invalid flag use
- **WHEN** the flag is repeated, has an attached value or appears without database mode before the script path
- **THEN** validation fails before connecting.

#### Scenario: Script argument remains unchanged
- **WHEN** the flag follows the script path
- **THEN** it remains a script argument and does not enable decompilation.
