## ADDED Requirements

### Requirement: Reached configuration root access rights
AccessRight SHALL accept the current acquired Metadata root for the reached SaveUserData and Administration rights, including Russian names. Its existing host request SHALL identify the acquired configuration GUID and name. An unsupported or inapplicable root right SHALL raise even in a privileged frame.

#### Scenario: Original UNF settings access
- **WHEN** original code checks SaveUserData on Metadata
- **THEN** the request targets the acquired configuration root instead of raising a metadata type error
- **AND** the session's role grants determine the ordinary current-principal result

#### Scenario: Validated privileged current principal
- **WHEN** a valid supported root right is checked for the current principal in a privileged frame
- **THEN** the result is true
- **AND** an unknown or inapplicable right still raises

### Requirement: Explicit root right subjects retain their role grants
An explicit role SHALL be evaluated directly in both modes. An explicit user SHALL require the current principal's Administration right outside privileged mode, including an explicit current user. Privileged mode SHALL bypass that administrative gate but SHALL NOT replace the explicit subject's grants with true. Missing acquired user or role data SHALL fail explicitly.

#### Scenario: Explicit denied role
- **WHEN** a denied role is checked in a privileged frame
- **THEN** the result remains false

#### Scenario: Explicit current user without Administration
- **WHEN** a non-administrative principal explicitly supplies its own user description
- **THEN** ordinary mode raises an administrative-access error
- **AND** privileged mode evaluates that user's actual role grants

#### Scenario: Missing rights resource
- **WHEN** an explicit subject needs an unread or unreadable role
- **THEN** checking raises instead of reporting a grant or a refusal

### Requirement: Root metadata identity is distinct from the catalog namespace
The host SHALL be able to supply the configuration metadata object's identity independently of the existing catalog namespace identity. Acquisition SHALL preserve the native root identity used by role resources without changing the namespace or its type declarations. Missing root metadata identity SHALL fail a root right check explicitly.

#### Scenario: Different identities in a native configuration
- **WHEN** the catalog namespace and configuration metadata GUID differ
- **THEN** the access request uses the configuration metadata GUID
- **AND** the catalog namespace and existing type identities remain unchanged

#### Scenario: Missing acquired root identity
- **WHEN** a host omits the configuration metadata identity
- **THEN** a root right check raises instead of using the catalog namespace as a rights target
