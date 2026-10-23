## ADDED Requirements

### Requirement: Document attribute metadata rights targets
AccessRight and VerifyAccessRights SHALL accept an acquired ordinary document attribute metadata value from the current immutable catalog and delegate it through the existing Session data adapter. The request SHALL expose the owning document metadata and the acquired attribute descriptor through AccessRightRequest::attribute() -> Option<&DocumentAttribute>, preserving the attribute identity and name. The request SHALL retain the original BSL metadata value for adapter diagnostics and journal payloads. Existing owner/root requests SHALL expose no attribute descriptor and preserve their behavior. Attribute targets SHALL preserve the existing subject and optional standard-path arguments.

#### Scenario: Attribute View reaches the adapter
- **WHEN** code passes Metadata.Documents.<Document>.Attributes.<Attribute> to AccessRight with View
- **THEN** the adapter receives the current principal, owning document and exact acquired attribute descriptor
- **AND** the returned grant comes from that adapter rather than a host-side child-right evaluator

#### Scenario: Verification preserves the child value
- **WHEN** code passes the acquired attribute to VerifyAccessRights
- **THEN** the adapter receives the same target identities and original child metadata value
- **AND** the child name and semantic identity remain available in retained request data

#### Scenario: Explicit subject and parent requests
- **WHEN** AccessRight receives an explicit acquired role or user for an attribute target
- **THEN** that subject remains distinct from the current principal in the adapter request
- **AND** a later check on the owning document exposes no attribute descriptor

### Requirement: Attribute rights target isolation and unsupported policy
Attribute metadata from another catalog SHALL be rejected before calling the Session adapter. Attribute property reads and semantic identity SHALL retain existing behavior. A database adapter that does not implement acquired attribute rights SHALL report an explicit unsupported error; it SHALL NOT interpret the child as its owning document and grant the owner's Read right, including in privileged mode. Actual applicability, role hierarchy and denial-event policy SHALL remain owned by the rights-aware adapter/SDK.

#### Scenario: Foreign catalog attribute
- **WHEN** a host supplies an attribute metadata value from a different catalog to a Session rights operation
- **THEN** the operation rejects the target before adapter invocation

#### Scenario: Owner Read does not grant unsupported attribute Read
- **GIVEN** the acquired role grants Read on a document and the database adapter lacks attribute-right support
- **WHEN** ordinary or privileged code checks Read on the document's attribute metadata
- **THEN** the adapter explicitly reports unsupported attribute rights rather than returning the document grant
- **AND** checking the document itself retains its existing result
