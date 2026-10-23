## ADDED Requirements

### Requirement: Host collection iteration is independent of public indexing
The object protocol exported through open-bsl SHALL allow a host collection to provide elements for For Each independently of its ordinary indexed access. The default implementation SHALL preserve the existing indexed-collection behavior.

#### Scenario: A non-indexable iterable host collection
- **GIVEN** a host object supplies a collection length and iteration elements but rejects indexed access
- **WHEN** BSL executes For Each over that object
- **THEN** it visits the supplied elements in order
- **AND** ordinary numeric brackets still fail

#### Scenario: An existing indexed collection
- **GIVEN** an existing collection does not override iteration-element retrieval
- **WHEN** a previously working For Each program runs
- **THEN** its values, loop control and mutations have the same behavior as before

### Requirement: Existing loop execution remains the single implementation
Compiled For Each SHALL use the existing VM loop execution and SHALL NOT introduce a second BSL dispatcher or host-specific loop executor. The facade consumer SHALL be able to implement the extension without importing internal crate names.

#### Scenario: Compile and serialize a host loop
- **WHEN** a loop over the host collection is compiled, printed, parsed and executed
- **THEN** the loop preserves independent iteration access after the round trip

#### Scenario: External facade consumer
- **WHEN** the consumer implements the non-indexable iterable object
- **THEN** it builds using only the open-bsl facade
