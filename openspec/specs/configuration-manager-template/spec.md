# configuration-manager-template Specification

## Purpose
Define finite platform template-method declarations for configuration managers
and explicit failure when actual template resources cannot be retrieved.

## Requirements

### Requirement: Data processor manager template method declaration
Data processor managers SHALL declare ПолучитьМакет / GetTemplate with exactly one required argument through their existing method context. The unchanged manager module SHALL resolve the bare call without accepting arbitrary unresolved functions or affecting other manager kinds.

#### Scenario: Original manager module call
- **WHEN** a data processor manager module contains ПолучитьМакет("МакетОтчета")
- **THEN** the original source links through its declared receiver method
- **AND** the same method recognizes its English alias and rejects incorrect argument counts

### Requirement: Missing template implementation is explicit
Until real template retrieval is available, executing the declared method SHALL raise an explicit unavailable-template error rather than return a fabricated template.

#### Scenario: Actual template retrieval
- **WHEN** code executes the declared method without a real resource implementation
- **THEN** it fails explicitly
- **AND** no empty document or successful substitute result is returned

### Requirement: Document manager template method declaration
Document managers SHALL declare ПолучитьМакет / GetTemplate with one required argument through their existing finite method context. Bare original manager calls SHALL link, and the declaration SHALL retain case-insensitive aliases and arity validation. Actual retrieval without a real resource implementation SHALL fail explicitly rather than fabricate a template.

#### Scenario: Original document manager source
- **WHEN** unchanged document manager source calls ПолучитьМакет with a template name
- **THEN** the call resolves through its own declared receiver
- **AND** an actual unsupported retrieval produces an explicit unavailable-template error

#### Scenario: Existing processor behavior
- **WHEN** a data processor manager uses the previously declared method
- **THEN** its aliases, arity and existing diagnostics remain unchanged

### Requirement: Catalog manager template method declaration
Catalog managers SHALL declare ПолучитьМакет / GetTemplate with one required argument through their finite receiver context. Original bare manager calls SHALL link, aliases SHALL be case insensitive, and incorrect arity SHALL be rejected. Existing exported manager methods with the same name SHALL retain their behavior. Retrieval without real resource support SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Original catalog manager source
- **WHEN** an original catalog manager procedure calls ПолучитьМакет with a template name
- **THEN** the module links through its receiver context
- **AND** executing unsupported retrieval raises an explicit error

#### Scenario: Existing manager export
- **WHEN** a catalog manager already exports a procedure with the same name
- **THEN** the existing procedure remains callable with its existing behavior

### Requirement: Information register manager template method declaration
Information register managers SHALL declare ПолучитьМакет / GetTemplate with
one required argument through their finite receiver context. Original bare
manager calls SHALL link, aliases SHALL be case insensitive, and incorrect
arity SHALL be rejected. Existing exported manager methods with the same name
SHALL retain their arity and result. Retrieval without real resource support
SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Dormant original manager call
- **WHEN** an information register manager contains an unexecuted procedure calling ПолучитьМакет with one argument
- **THEN** the original module links through its own receiver context
- **AND** the declaration does not introduce a global function outside that context

#### Scenario: Actual retrieval
- **WHEN** code executes either alias without real template resource support
- **THEN** it raises an explicit unsupported-template error
- **AND** it returns no empty document or fabricated value

#### Scenario: Existing export and variable precedence
- **WHEN** a manager exports a same-named function or declares a same-named variable
- **THEN** the existing export retains its arity and result
- **AND** reading the variable preserves its existing value

#### Scenario: Invalid argument count
- **WHEN** a receiver method call supplies zero or two arguments without a same-named export
- **THEN** normal semantic arity validation rejects it

### Requirement: Accumulation register manager template method declaration
Accumulation register managers SHALL declare ПолучитьМакет / GetTemplate with exactly one required argument through their existing finite receiver context. Aliases SHALL be case insensitive. The declaration SHALL preserve existing same-named exports and variable values and SHALL NOT introduce a global function. Actual retrieval without real resource support SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Dormant original call
- **WHEN** an unchanged accumulation register manager contains an unexecuted method calling either template alias with one argument
- **THEN** compilation links the call through the manager receiver context
- **AND** no template is retrieved

#### Scenario: Explicit unsupported retrieval
- **WHEN** execution invokes either alias without a same-named export or resource implementation
- **THEN** it raises an explicit unsupported-template error
- **AND** returns no fabricated value

#### Scenario: Existing export and variables
- **WHEN** a manager defines a same-named export or variable
- **THEN** the export retains its own arity and result
- **AND** reading the variable preserves its value

#### Scenario: Finite scope and arity
- **WHEN** a bare manager call supplies zero or two arguments without a same-named export
- **THEN** normal semantic arity validation rejects it
- **WHEN** unrelated code calls the name as a global function
- **THEN** the existing undefined-function diagnostic remains

### Requirement: Chart manager template method declaration
Chart-of-accounts managers SHALL declare ПолучитьМакет and GetTemplate with zero or one argument through their finite receiver context. Bare own-manager calls SHALL link. A zero-argument call SHALL return Undefined. Existing same-named manager exports SHALL retain their behavior. Retrieval with a template argument without real resource support SHALL fail explicitly rather than fabricate a document.

#### Scenario: Dormant original manager call
- **WHEN** original chart manager source contains a bare call with one template name
- **THEN** it links through the declared receiver context without declaring a global function

#### Scenario: Omitted argument
- **WHEN** either alias is called qualified or bare in the manager with zero arguments
- **THEN** it returns Undefined

#### Scenario: Excess arguments
- **WHEN** the receiver method receives two arguments without a same-named export
- **THEN** normal arity validation rejects the call

#### Scenario: Actual resource unsupported
- **WHEN** code requests a named template without a real resource implementation
- **THEN** it raises an explicit unavailable-template error and returns no substitute document

#### Scenario: Existing export
- **WHEN** the manager declares a same-named exported function
- **THEN** its existing arity and result remain available
