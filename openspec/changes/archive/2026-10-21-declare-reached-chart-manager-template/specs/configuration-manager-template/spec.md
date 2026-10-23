## ADDED Requirements

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
