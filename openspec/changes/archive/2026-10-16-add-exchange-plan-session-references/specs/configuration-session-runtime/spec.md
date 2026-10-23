## ADDED Requirements

### Requirement: Exchange-plan empty references for session initialization
An acquired exchange-plan manager SHALL provide zero-argument `ПустаяСсылка` / `EmptyRef` returning the empty reference of that exact acquired plan. The reference type SHALL be available through the Russian `ПланОбменаСсылка.<Name>` and English `ExchangePlanRef.<Name>` names. Catalog and owner identity SHALL remain part of the type identity.

#### Scenario: Initialize the integrated-system session parameter
- **WHEN** original initialization assigns an acquired exchange plan's EmptyRef to a matching session parameter
- **THEN** it receives that plan's empty typed reference without reading or writing a node

#### Scenario: Distinct acquired exchange plans
- **WHEN** empty references belong to different acquired plans
- **THEN** their reference types remain distinct and their values do not compare equal

### Requirement: Reached dormant exchange-plan node-creation declaration
The exchange-plan manager SHALL declare zero-argument `СоздатьУзел` / `CreateNode` as a function for the existing explicit and implicit receiver resolution. Actual invocation SHALL fail explicitly as unsupported; declaration SHALL NOT fabricate a node or persist data.

#### Scenario: Compile an original manager module
- **WHEN** a retained exchange-plan manager module contains a dormant unqualified CreateNode call
- **THEN** existing receiver resolution recognizes the documented manager method

#### Scenario: Execute node creation
- **WHEN** code actually invokes CreateNode
- **THEN** execution raises an explicit unsupported-operation error without returning a fabricated node

### Requirement: Reached current exchange-node declaration
The exchange-plan manager SHALL declare zero-argument `ЭтотУзел` / `ThisNode` for existing explicit and implicit receiver resolution. Actual invocation SHALL fail explicitly while the host has not acquired the current node; it SHALL NOT substitute an empty or invented reference.

#### Scenario: Compile the original IntegratedSystems manager
- **WHEN** the original retained manager contains an unqualified ThisNode call
- **THEN** the existing receiver context recognizes that documented function
- **AND** invoking it reports unsupported current-node access

### Requirement: Measured exchange-plan reference construction
An acquired exchange-plan manager SHALL provide `ПолучитьСсылку` / `GetRef` with zero or one argument through the existing configuration-reference mechanism. A supplied UUID SHALL be retained in that exact plan's typed reference without database I/O. An omitted or Undefined argument SHALL produce a fresh UUID. Invalid UUID argument types and excessive arity SHALL fail explicitly.

#### Scenario: Original manager constructs a reference from a fixed UUID
- **WHEN** the original manager calls GetRef with a UUID
- **THEN** the reference preserves that UUID and the manager's acquired owner type
- **AND** the zero UUID equals that owner's EmptyRef

#### Scenario: Construct a fresh reference
- **WHEN** GetRef is called without an argument or with Undefined
- **THEN** it returns a nonempty reference with a fresh UUID and that owner's type
