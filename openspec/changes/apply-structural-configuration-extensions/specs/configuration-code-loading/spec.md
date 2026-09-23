## ADDED Requirements

### Requirement: Structural configuration extension application
`bsl-config-db` SHALL build the neutral snapshot from the configuration
resources acquired through `open-sdbl-db`. It SHALL reuse the database
adapter's provider handling, limits, resource-part assembly, metadata
acquisition, and extension resource acquisition. It SHALL NOT implement SQL
provider access independently.

Active extensions SHALL be applied in platform order through explicit
extension-to-base identity links. An extension MAY add objects and modules or
replace the effective module of a borrowed object. The result SHALL contain one
effective source body per logical module and SHALL expose neither base/patch
variants nor extension storage identities to consumers. Inactive extensions
SHALL NOT modify the code graph. Unknown activity, an incomplete catalog, or an
operation that cannot be reduced to an effective module graph SHALL fail with
an error identifying the extension.

#### Scenario: Structural active extension
- **WHEN** an active extension adds a module and changes a module of an explicitly borrowed base object
- **THEN** both changes appear in the one effective module graph
- **AND** the borrowed object is linked by identity rather than by name

#### Scenario: Ordered active extensions
- **WHEN** two active extensions change the same logical module
- **THEN** they are applied in platform order and the final snapshot contains only the resulting source body

#### Scenario: Inactive structural extension
- **WHEN** a structural extension is present but inactive
- **THEN** its modules do not enter the effective code graph

#### Scenario: Unsupported structural operation
- **WHEN** an active extension operation cannot be represented as one effective module graph
- **THEN** loading fails explicitly and base sources are not presented as effective
