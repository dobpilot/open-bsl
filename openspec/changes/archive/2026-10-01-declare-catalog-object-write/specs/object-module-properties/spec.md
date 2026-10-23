## ADDED Requirements

### Requirement: Declared catalog object write boundary
The existing catalog object receiver SHALL declare Записать / Write with zero arguments. Original modules containing this method SHALL compile through the existing receiver method mechanism. A previously callable exported object-module procedure named Записать / Write SHALL retain its behavior; this addition SHALL NOT shadow it. Until persistence is connected, invocation SHALL fail explicitly without successful no-op behavior, database writes or a fabricated persisted reference.

#### Scenario: Uncalled procedure contains Write
- **WHEN** an original object module declares an exported procedure containing bare Записать()
- **THEN** the existing interpreter can compile the declared procedure
- **AND** ordinary local and module name priority remains unchanged

#### Scenario: Persistence is invoked before integration
- **WHEN** configuration code invokes Записать or Write on a catalog object
- **THEN** it receives an explicit persistence-not-connected failure
- **AND** the object's persisted reference and new-object status remain unchanged

#### Scenario: Wrong arity
- **WHEN** the method receives an argument
- **THEN** the existing method arity checks reject the call

### Requirement: Read-only configuration export inspection
CallContext SHALL expose configuration_has_export(module, name) returning RtResult<bool> through the existing executor service. It SHALL inspect compiled exported names using case-insensitive comparison without executing a module body or procedure. Missing executor or unavailable module information SHALL fail explicitly.

#### Scenario: Existing exported Write remains callable
- **WHEN** an object module exports a procedure named Записать
- **THEN** adding the platform Write declaration does not intercept that procedure
- **AND** the component can inspect export presence without executing it

#### Scenario: Missing or nonexported name
- **WHEN** a compiled module has no exported procedure with the inspected name
- **THEN** inspection returns false without running the module
