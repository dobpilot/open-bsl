## ADDED Requirements

### Requirement: Reuse a compiled session factory for another runtime kind
SessionFactory SHALL implement Clone and expose with_kind(self, SessionKind) -> Self. Cloning and selecting a kind SHALL reuse the compiled engine without compiling configuration again. The selected kind SHALL apply to future sessions only. The original factory and already opened sessions SHALL remain unchanged. New sessions SHALL retain independent state, host data, parameters and explicitly supplied connections. Existing open_background checks and parent-parameter copying SHALL remain effective.

#### Scenario: Another client kind
- **WHEN** a host clones a thin-client factory and selects WebClient
- **THEN** sessions from the clone report WebClient and sessions from the original retain ThinClient
- **AND** both factories reuse the compiled engine while their module variables remain independent

#### Scenario: Background child
- **WHEN** a host clones the parent factory, selects BackgroundJob and calls open_background with another connection and the real parent
- **THEN** the child gets a separate parameter container with copied parent values
- **AND** subsequent child changes do not change parent parameters

#### Scenario: Existing opening guard
- **WHEN** open_background is called on a factory that is not configured for BackgroundJob
- **THEN** the existing NotBackgroundFactory error is retained
