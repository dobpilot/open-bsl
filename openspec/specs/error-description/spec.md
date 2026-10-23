# error-description Specification

## Purpose
Define native-measured handler descriptions, explicit handler extents and isolation across calls, dynamic snippets, async execution and configuration objects.

## Requirements

### Requirement: Description of the current call's active handler
The existing zero-argument `ОписаниеОшибки` and `ErrorDescription` names SHALL return a string describing the exception handled by the active handler of the current call. The names SHALL remain case-insensitive aliases. The implementation SHALL reuse the existing exception representation.

#### Scenario: Explicit exception message
- **WHEN** a handler catches an exception with the explicit message `outer-probe`
- **THEN** its error description contains that message
- **AND** obtaining the description does not raise an unsupported-capability error

#### Scenario: No active handler
- **WHEN** the current call has no active exception handler
- **THEN** its error description is the empty string

### Requirement: Nested handler restoration
A nested handler SHALL observe its own exception. After the nested handler finishes, the enclosing active handler SHALL again observe its original exception. After the enclosing handler finishes, its description SHALL no longer be active.

#### Scenario: Nested exceptions
- **GIVEN** an outer handler for `outer-probe`
- **WHEN** a nested handler catches `inner-probe` and then finishes
- **THEN** the nested handler's description contains `inner-probe`
- **AND** the outer handler's subsequent description contains `outer-probe` and not `inner-probe`
- **AND** the description after the outer handler is empty

### Requirement: Call and session isolation
An ordinary called function SHALL NOT inherit the caller's active error description. Independent execution states and sessions SHALL NOT share handler descriptions.

#### Scenario: Ordinary function called from a handler
- **GIVEN** a caller handling `outer-probe`
- **WHEN** it calls a separate function which has no active handler
- **THEN** the called function observes an empty error description
- **AND** returning to the caller does not discard the caller's active handler

#### Scenario: Independent states
- **GIVEN** one state has handled an exception
- **WHEN** another independent state obtains an error description without an active handler
- **THEN** the other state observes an empty string

### Requirement: Explicit serialized handler extent
Each serialized exception handler SHALL carry an explicit exclusive end of its body in addition to the existing protected range and handler entry. The reader and verifier SHALL reject incompatible format versions and handler extents outside the chunk or ending before their entry. Instruction relocation SHALL preserve all handler boundaries. The representation SHALL permit an empty handler at the end of a chunk.

#### Scenario: Handler extent round trip
- **WHEN** a compiled nested-handler program is printed and parsed as textual bytecode
- **THEN** both protected ranges and handler body extents are preserved

#### Scenario: Invalid handler extent
- **WHEN** an image declares a handler end before its entry or beyond the chunk
- **THEN** it is rejected before execution

#### Scenario: Empty handler
- **WHEN** a handler entry and its exclusive end both equal the chunk length
- **THEN** that empty handler remains valid

### Requirement: Dynamic snippet isolation
`Выполнить` and `Вычислить` SHALL NOT inherit the surrounding call's active error description. Returning from a dynamic snippet SHALL preserve the surrounding call's active handler.

#### Scenario: Dynamic evaluation in a handler
- **GIVEN** the surrounding call handles `outer-probe`
- **WHEN** it evaluates `ОписаниеОшибки()` through `Вычислить` or assigns its result through `Выполнить` without a snippet-local handler
- **THEN** the dynamic code observes an empty description
- **AND** the surrounding handler subsequently observes `outer-probe`

### Requirement: Async call handler isolation
An async called function SHALL have its own active handler description. Awaiting an operation within a handler SHALL preserve that handler's description, and awaiting a child call SHALL preserve the parent's active handler.

#### Scenario: Child with an awaited file operation
- **GIVEN** an async caller handles `parent-probe`
- **WHEN** its child observes a description before and after awaiting a file existence operation without an active child handler
- **THEN** both child descriptions are empty
- **AND** the child can observe its own `child-probe` before and after an await inside its own handler
- **AND** the parent still observes `parent-probe` after awaiting the child

### Requirement: Configuration object call boundaries
A configuration object's module initialization and exported method calls SHALL have their own handler-description scope. Handling an exception inside an object method SHALL NOT replace the caller's active handler description. An exception escaping an object method SHALL be observable in the caller's catching handler, after which an enclosing caller handler SHALL be restored.

#### Scenario: Object created from a handler
- **GIVEN** the caller handles `caller-probe`
- **WHEN** it creates an in-memory data processor and calls its exported description function
- **THEN** module initialization and the exported function observe empty descriptions without their own handlers
- **AND** an object-local handler can observe its own exception
- **AND** returning from that handler preserves `caller-probe`

#### Scenario: Exception escapes an exported method
- **GIVEN** the caller's outer handler handles `caller-probe`
- **WHEN** an exported object method raises `object-thrown-probe` into a nested caller handler
- **THEN** that nested handler observes `object-thrown-probe`
- **AND** the outer caller handler subsequently observes `caller-probe`
