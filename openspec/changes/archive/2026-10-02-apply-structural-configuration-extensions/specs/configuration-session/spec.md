## ADDED Requirements

### Requirement: Acquired extension description completeness
The ORM catalog SHALL retain a complete provider-neutral description of every
acquired database extension: identity, name, version, hash, safe-mode settings,
distributed-infobase use, activity, scope and platform order. Applied and
disabled session collections SHALL be derived from explicit acquired activity.
No missing property SHALL be replaced with an inferred platform default.

#### Scenario: Active and inactive acquired extensions
- **WHEN** acquisition returns one active and one inactive extension with complete properties
- **THEN** both appear in the database collection
- **AND** the active extension appears only in the applied collection and the inactive extension only in the disabled collection

#### Scenario: Incomplete extension description
- **WHEN** acquisition omits a property required by the public extension object
- **THEN** catalog construction fails explicitly instead of fabricating a value
