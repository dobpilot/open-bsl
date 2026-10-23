## ADDED Requirements

### Requirement: Missing imported methods fail at common-module method entry
With the existing common-module name-error policy enabled, an unresolved
method call to a known imported common module in a common-module function or
procedure body SHALL NOT prevent valid neighboring methods from running.
Calling the affected method SHALL raise a catchable error before its body
or call arguments execute. Valid neighboring methods SHALL remain usable.

#### Scenario: Native missing-method receipt
- **WHEN** a valid parent export is called before and after an export that references a missing child method
- **THEN** both valid calls return their native result
- **AND** the affected call raises a catchable error without either body or argument markers

#### Scenario: Strict default
- **WHEN** the common-module opt-in is disabled
- **THEN** the same unresolved imported method retains its existing semantic error

#### Scenario: Other source contexts
- **WHEN** an object, manager or explicit session module contains the same unresolved imported call
- **THEN** its existing strict diagnostics remain unchanged

#### Scenario: Unrelated unknown names
- **WHEN** a common module references an unknown bare function or variable
- **THEN** its existing diagnostics and deferred whole-module policy remain unchanged
