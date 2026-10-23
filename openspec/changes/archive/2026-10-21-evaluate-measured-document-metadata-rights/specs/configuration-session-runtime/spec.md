## ADDED Requirements

### Requirement: Measured Document metadata rights use acquired SDK policy

The database adapter SHALL evaluate retained Document owner, Number/Номер, Date/Дата and ordinary own-attribute Read/View targets using acquired SDK metadata and role resources. It SHALL validate target structure and applicability before privilege bypass. Existing explicit subject selection and current-principal isolation SHALL remain in effect.

#### Scenario: Child Read is not an owner grant
- **WHEN** ordinary or privileged code checks Read for a retained Document standard or ordinary attribute
- **THEN** AccessRight reports the measured applicability error
- **AND** VerifyAccessRights succeeds without a denial event

#### Scenario: Acquired View policy
- **WHEN** a retained role profile checks View on an acquired Document child
- **THEN** AccessRight and VerifyAccessRights follow their respective measured owner, explicit-child, default and independent-child rules

#### Scenario: Invalid acquired membership
- **WHEN** an attribute is not in the complete acquired owner membership or its descriptor is missing or ambiguous
- **THEN** both ordinary and privileged calls fail explicitly before evaluating role grants

### Requirement: Document verification retains the Session denial event

A supported Document VerifyAccessRights refusal SHALL use the current Session event-log writer, preserving the original metadata target and existing denial payload and transaction mode. Applicability success or structural rejection SHALL NOT emit a false access-denial event.

#### Scenario: Ordinary attribute refusal
- **WHEN** verification of View on a retained ordinary attribute is denied
- **THEN** exactly one existing AccessDenied event retains that child metadata value
- **AND** execution raises the measured access-denial error

#### Scenario: Existing Catalog policy
- **WHEN** existing Catalog/root controls run after Document integration
- **THEN** their grants, applicability and denial events remain unchanged

#### Scenario: Attribute with an additional standard path
- **WHEN** an ordinary attribute target also supplies a nonempty standard-attribute path
- **THEN** the adapter reports explicit unsupported behavior before applicability or privilege handling
- **AND** it does not discard the supplied path
