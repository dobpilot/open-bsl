## ADDED Requirements

### Requirement: Preserve scalar UUID query parameters and results
The PostgreSQL database adapter SHALL accept an existing BSL UUID as a typed SDK UUID parameter and return a typed SQL UUID cell as the same BSL UUID value. Physical storage byte order SHALL remain the SDK responsibility. Raw binary cells SHALL NOT be guessed as UUIDs and reference values SHALL retain their existing interpretation.

#### Scenario: Session user query
- **WHEN** original session initialization filters infobase users using the current user's UUID through UNION ALL
- **THEN** the adapter passes the parameter and returns matched UUID values with their BSL UUID identity intact

#### Scenario: Mismatched cell
- **WHEN** a binary cell is supplied for a UUID result column or a UUID cell for a different kind
- **THEN** the adapter reports a type mismatch instead of interpreting arbitrary bytes as UUID
