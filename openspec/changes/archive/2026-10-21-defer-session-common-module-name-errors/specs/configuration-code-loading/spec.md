## ADDED Requirements

### Requirement: Distinct fatal runtime compilation error
The runtime SHALL expose `RtError::ModuleCompilation(String)` as a distinct error carrying the compilation diagnostic. It SHALL bypass BSL exception handlers through the existing VM error classification boundary. This classification SHALL NOT change existing runtime error classes or State isolation.

#### Scenario: Host returns a compilation error during module initialization
- **WHEN** a component call returns that error from a module initializer inside nested BSL exception handlers
- **THEN** neither handler nor subsequent caller code runs, and the host receives the typed diagnostic
- **AND** an unrelated State of the same engine remains usable

### Requirement: Host-controlled deferred common-module name failures
The facade SHALL provide an explicit opt-in to defer undefined-variable semantic failures and undefined global-function failures in method bodies of named and global configuration common modules until the affected module instance is initialized. Existing facade builder defaults and resolver entry points SHALL retain strict semantic rejection. The option SHALL NOT defer semantic validation of private object/manager modules or explicit session modules. The implementation SHALL use the existing resolver, compiler, and module initialization machinery.

#### Scenario: Unused erroneous child common module
- **GIVEN** the host enables the policy and a valid parent export has a neighboring export referencing a child with an undefined variable
- **WHEN** execution calls only the valid parent export
- **THEN** engine construction and the parent call succeed without initializing the child

#### Scenario: Erroneous common module is accessed
- **GIVEN** the host enables the policy and an affected common module contains an undefined variable
- **WHEN** execution first accesses that module
- **THEN** initialization fails before any export body of that module runs
- **AND** the compilation failure is not converted to an ordinary exception recoverable by the caller's BSL `Попытка/Исключение`

#### Scenario: Default compilation remains strict
- **WHEN** an existing consumer builds a configuration without enabling the policy
- **THEN** an undefined variable retains its existing compile-time diagnostic

#### Scenario: Caller and argument side effects precede child compilation failure
- **GIVEN** the host enables the policy and the child contains an undefined variable
- **WHEN** a parent method performs a side effect and computes a side-effecting argument before calling the child
- **THEN** both caller and argument side effects occur before the fatal child compilation failure
- **AND** the child export body does not run

#### Scenario: Private module diagnostics remain strict
- **WHEN** the policy is enabled and a selected private object or manager module contains an undefined variable
- **THEN** engine construction rejects that module with its existing semantic diagnostic

### Requirement: Session factories use deferred common-module name failures
Session factories SHALL enable the host policy for server common modules in their configuration graph. They SHALL preserve module instance isolation between Sessions and SHALL NOT rewrite source text or fabricate a missing module.

#### Scenario: Faulty common module is not reached during startup
- **GIVEN** an erroneous common module belongs to the conservative static closure but startup never accesses it
- **WHEN** a Session initializes its parameters
- **THEN** that module does not prevent initialization of unrelated modules

#### Scenario: Unused common module with an unknown global function
- **GIVEN** the host enables the policy and a child common module has a method body calling an unknown global function
- **WHEN** execution calls only a valid parent export which does not access that child
- **THEN** construction and the parent call succeed
- **AND** first access to the child fails with a fatal module compilation diagnostic before any child export body runs

#### Scenario: Missing imported method retains its existing policy
- **GIVEN** a common-module method calls an absent method of an existing imported common module
- **WHEN** another valid export of that module is called
- **THEN** the existing method-scoped failure policy is preserved
