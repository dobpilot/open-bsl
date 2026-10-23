## ADDED Requirements

### Requirement: Active call context for standard settings loads
The standard settings manager SHALL pass the current call context to a context-aware adapter load callback. The callback SHALL default to the existing load operation so existing adapters retain their behavior. The context SHALL belong to the current Session and SHALL NOT be retained or shared across Sessions by the runtime.

#### Scenario: Context-aware adapter
- **WHEN** an adapter overrides the context-aware load callback
- **THEN** Load invokes it with the current Session principal, request and active context

#### Scenario: Existing adapter
- **WHEN** an adapter only implements the existing load callback
- **THEN** Load retains that callback's result through default delegation
