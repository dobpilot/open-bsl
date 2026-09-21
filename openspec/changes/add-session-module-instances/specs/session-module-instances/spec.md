## ADDED Requirements

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
