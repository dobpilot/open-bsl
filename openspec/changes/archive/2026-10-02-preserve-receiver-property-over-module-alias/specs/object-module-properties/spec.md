## ADDED Requirements

### Requirement: Receiver property priority over imported aliases
A declared receiver property SHALL take priority over a same-named imported common-module alias in reads and qualified method calls. Parameters and explicit local or module variables SHALL retain priority over that property. Modules without receiver property context SHALL retain existing imported-module resolution.

#### Scenario: Colliding property method
- **WHEN** an object module calls Количество on its ДополнительныеСвойства property and a same-named imported module exports Количество returning 999
- **THEN** the call SHALL use the receiver property and match explicit ЭтотОбъект access

#### Scenario: Parameter shadows property
- **WHEN** the object method receives a same-named parameter containing a one-element structure
- **THEN** calling Количество SHALL return 1 from the parameter

#### Scenario: No receiver context
- **WHEN** the same alias is used in a module without a colliding receiver property
- **THEN** the imported module export SHALL remain callable
