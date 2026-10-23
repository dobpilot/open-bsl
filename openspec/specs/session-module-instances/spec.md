# session-module-instances Specification

## Purpose
Define the facade lifecycle for initialized per-State module instances and host calls to their exported procedures and functions.

## Requirements

### Requirement: Explicit session module instances
The facade SHALL provide an explicit module instance belonging to one State. Creation SHALL execute the module body once and publish the instance only after successful initialization. A top-level return SHALL finish initialization and its value SHALL be discarded. Repeated acquisition of the same compiled module in a State SHALL refer to its initialized instance. Calls SHALL preserve module variables. Separate States SHALL not share module state. Existing run and start SHALL keep their independent-run behavior.

#### Scenario: Initialize once and call repeatedly
- **WHEN** a host acquires an instance and calls an export twice
- **THEN** the body runs once and the second call sees changes from the first

#### Scenario: Failed initialization
- **WHEN** the module body fails
- **THEN** no instance is returned

#### Scenario: Foreign session
- **WHEN** a host uses an instance with another State
- **THEN** the operation fails without executing its code

#### Scenario: Foreign engine or configuration
- **WHEN** a host tries to instantiate a module from another engine or configuration
- **THEN** creation fails before executing code

### Requirement: Session common modules
Explicit instances in the same State SHALL use the same session instances of configuration common modules. Separate States SHALL have independent common module instances.

#### Scenario: Two entry modules
- **WHEN** entry A changes a common module variable and entry B reads it in the same State
- **THEN** B observes the change
- **AND** an entry in another State does not observe it

### Requirement: Export invocation
The host SHALL invoke only exported procedures or functions, using case-insensitive names and an exact argument count. A successful call SHALL return the result and final values of every parameter, including by-value parameters. A failed call SHALL return an error without parameter results or rollback. Ordinary BSL exceptions SHALL leave the instance callable. The execution capabilities SHALL match ordinary execution, including components, host services, dynamic code, common modules and asynchronous operations.

#### Scenario: Export and argument checks
- **WHEN** a name is unknown, not exported, or supplied with a wrong argument count
- **THEN** the call fails without executing the target

#### Scenario: Mutate then throw
- **WHEN** an export mutates state and raises a BSL exception
- **THEN** a later call observes the mutation and can complete successfully

#### Scenario: Full execution services
- **WHEN** an export uses dynamic code, common modules or asynchronous host operations
- **THEN** those operations use the same services and scheduling semantics as ordinary execution

### Requirement: Values retain field identity within one State
Values passed as arguments, returned results or final parameters among explicit module instances of one State SHALL retain field spellings and mutable object identity regardless of each module's compiled name-table order. Changing an unrelated field access in another module SHALL NOT change observable field lookup. This guarantee SHALL NOT imply cross-State value transfer or change independent run/start ownership.

#### Scenario: Shifted field-name tables
- **WHEN** entry A creates a Structure with field Right, its compiled field table has an unrelated preceding name, and entry B receives that value in the same State
- **THEN** B reads Right by its original spelling and case variants
- **AND** A continues to read the same value

#### Scenario: Mutation through another entry
- **WHEN** two explicit entries in one State hold the same Structure or nested aliases and one entry changes or inserts a field
- **THEN** the other observes that mutation under the original field name
- **AND** cycles and repeated references are preserved rather than copied into independent values

### Requirement: Runtime field names survive instance calls
Names introduced while executing an explicit instance SHALL remain valid for its retained module values and values passed to other instances in that State across later calls, including after an ordinary BSL exception.

#### Scenario: Dynamic field followed by another call
- **WHEN** a call inserts a runtime-computed Structure field, then a later call in either explicit entry reads it
- **THEN** the field is resolved by its spelling without depending on unrelated name insertion order

#### Scenario: Mutate and raise
- **WHEN** an export inserts a field and raises an ordinary BSL exception
- **THEN** a later permitted call observes the mutation under that name

### Requirement: Session binding preserves compiled modules
Binding an explicit instance SHALL preserve the host's immutable compiled Module, its serialized representation and shared immutable configuration catalog tables. Different States SHALL keep independent runtime names and values. Existing independent run/start semantics and facade-only embedding SHALL remain available without new normal dependencies in the runtime or VM.

#### Scenario: One compiled module in different States
- **WHEN** the same compiled Module is instantiated in two States with different prior runtime name additions
- **THEN** each State executes correctly without changing the other's names or values
- **AND** serializing the original Module produces the unchanged representation

#### Scenario: Name binding exceeds the existing operand capacity
- **WHEN** a State-bound open-member name cannot fit the bytecode operand's existing index range
- **THEN** binding fails explicitly before executing the module body
- **AND** no identifier is truncated or silently interpreted as another name
