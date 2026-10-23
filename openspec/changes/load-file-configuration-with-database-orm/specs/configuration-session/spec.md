## MODIFIED Requirements

### Requirement: Database acquisition adapter
When database code mode is selected, `bsl-config-db` SHALL derive the source-neutral code snapshot and immutable ORM metadata catalog from one acquired database configuration. Both results SHALL carry the same configuration identity. When an XML or EDT source bundle is selected, the adapter SHALL supply the database ORM facts and session services independently of that bundle, without loading database BSL bodies or source-only form/template payloads. Reads of database descriptions required for ORM identity, physical mappings, authorization and data separation SHALL remain permitted and required. Catalog construction SHALL preserve the configuration objects, roles, session parameters, root properties, and complete extension snapshot required by session startup, and SHALL happen once before a reusable `SessionFactory` is built.

The adapter SHALL convert information-base users into `PrincipalCandidate` values using `InfoBaseUser::can_authenticate()` as the authentication decision and preserving the user's stable identifier, name, and assigned role identifiers. `bsl-orm` and `bsl-session` SHALL NOT parse database storage formats.

For session startup, the adapter SHALL select the `Config` resources needed
by its code and ORM projections after reading the configuration root. It
SHALL NOT retain unrelated `Config` resources merely to discard them after
acquisition.

#### Scenario: One acquisition supplies factory inputs
- **WHEN** a database configuration is loaded for session execution with database code selected
- **THEN** its code snapshot and ORM catalog are derived from that acquisition
- **AND** `SessionFactory` accepts their matching configuration identities

#### Scenario: Large unrelated resources
- **WHEN** `Config` contains forms or templates that neither factory input reads
- **THEN** the startup acquisition does not retain their compressed bytes
- **AND** the code and ORM projections still receive every resource they require

#### Scenario: Selectable database user
- **WHEN** an acquired user has an enabled authentication method and assigned roles
- **THEN** the adapter produces a selectable principal candidate with the same identifier, name, and role identifiers

#### Scenario: User without authentication
- **WHEN** an acquired user has no enabled authentication method
- **THEN** the adapter preserves the candidate but marks it as not selectable
- **AND** it does not infer eligibility from administrative or display flags

#### Scenario: Database ORM with file sources
- **WHEN** a matching XML or EDT bundle is selected for configured execution
- **THEN** the database adapter supplies real ORM storage facts, users, rights and session data access
- **AND** module bodies and source-owned form/template payloads are obtained only from that file bundle
- **AND** the existing selective database acquisition and per-session connection isolation are preserved

## ADDED Requirements

### Requirement: Mixed-source reusable session factory
The existing session factory SHALL compose one selected file-source bundle with
a database-backed ORM catalog and existing session resources after native
configuration identity and persistent owner compatibility checks. It SHALL use
the existing open-bsl resolver, compiler, module instances and VM, and SHALL NOT
rewrite source text or build an independent BSL implementation. Source-owned
metadata facets SHALL be attached to existing metadata/artifact consumers,
without registering a competing global Метаданные. Database-only constructors
and source selection defaults SHALL retain their existing behavior.

#### Scenario: Local source executes against live data
- **GIVEN** local BSL for a method differs from its database copy but has compatible ownership
- **WHEN** a session initializes and calls that method in file mode
- **THEN** open-bsl executes the selected local text
- **AND** ORM reads and queries use the real database connection and its acquired storage mappings

#### Scenario: Several sessions share a source bundle
- **WHEN** a factory opens two sessions against one selected file bundle
- **THEN** immutable acquired code and artifacts can be shared
- **AND** connections, principal state, module variables and session parameters remain isolated under the existing session contract

#### Scenario: File metadata does not replace database authorization
- **GIVEN** file role descriptions differ from the roles assigned to the database principal
- **WHEN** file-backed code requests database data
- **THEN** existing database-acquired authorization, RLS and separation decisions apply
- **AND** file descriptions do not grant a new role or remove a storage restriction
