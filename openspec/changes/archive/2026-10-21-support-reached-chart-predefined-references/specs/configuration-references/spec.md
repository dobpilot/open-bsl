## ADDED Requirements

### Requirement: Chart predefined account references
An acquired chart-of-accounts manager SHALL expose its declared predefined account names as read-only properties and as bare properties in its own manager module. Local and module variables SHALL retain their existing precedence. Resolution SHALL return the actual record reference through the current session adapter, never a fabricated reference derived from the predefined metadata GUID.

#### Scenario: Bare and qualified account identity
- **WHEN** the owning manager reads a declared predefined account both bare and through ПланыСчетов or ChartsOfAccounts
- **THEN** both reads and the corresponding query VALUE expression return equal chart references

#### Scenario: Qualified reference type
- **WHEN** the reference type is looked up as ПланСчетовСсылка or ChartOfAccountsRef qualified by its acquired owner name
- **THEN** both names resolve to the type returned by TypeOf for that account reference

#### Scenario: Read-only property
- **WHEN** code assigns a predefined account property on the manager
- **THEN** the existing read-only property error is returned

#### Scenario: Existing variable precedence and scope
- **WHEN** a local or module variable shadows a predefined account name
- **THEN** the variable wins
- **AND** an unrelated module does not acquire the bare account name

#### Scenario: Unresolved account
- **WHEN** the session adapter cannot resolve the declared account record
- **THEN** the existing explicit missing-record or adapter error is preserved

### Requirement: Chart empty reference receiver method
A chart-of-accounts manager SHALL provide ПустаяСсылка and EmptyRef with zero arguments, both qualified and as declared receiver methods in its own manager module. The result SHALL have the owner's chart reference type and nil UUID, compare equal across aliases, and report empty through Пустая and IsEmpty without a database read.

#### Scenario: Empty reference in manager context
- **WHEN** the owning chart manager calls either alias bare or qualified
- **THEN** the results are equal empty references of that chart's declared type
- **AND** they differ from its filled predefined account reference
