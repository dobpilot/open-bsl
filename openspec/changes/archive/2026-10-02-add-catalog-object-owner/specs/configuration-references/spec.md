## ADDED Requirements

### Requirement: Acquired catalog owner collection
The configuration catalog SHALL retain the acquired optional ordered owner-ID collection. Unknown owners SHALL remain distinct from a known empty collection. The database adapter SHALL use the existing SDK acquisition rather than infer owners from names or SQL rows.

#### Scenario: Known and unknown owner acquisition
- **WHEN** the SDK supplies a catalog's owner IDs
- **THEN** the ORM SHALL retain every acquired ID in order
- **WHEN** owner acquisition is unknown
- **THEN** accessing Owner SHALL fail explicitly without preventing unrelated object operations

### Requirement: Native new catalog object Owner
New catalog objects SHALL expose Владелец and Owner as aliases. With one acquired catalog owner, the initial value SHALL be its empty reference. With multiple acquired owners, the initial value SHALL be Undefined. With a known empty owner collection, reading SHALL return Undefined and assignment SHALL fail.

#### Scenario: Single owner
- **WHEN** a new object has exactly one acquired owner
- **THEN** its object-module body SHALL read the typed empty owner reference
- **AND** assignment during that body SHALL fail before object selection
- **AND** subsequent assignment of a compatible reference SHALL retain its identity, while an incompatible string SHALL become the typed empty reference

#### Scenario: Multiple owners
- **WHEN** a new object has two acquired catalog owners
- **THEN** its initial Owner SHALL be Undefined
- **AND** references of either acquired owner SHALL be accepted
- **AND** an incompatible string SHALL produce Undefined

#### Scenario: Object isolation
- **WHEN** one new object receives an owner reference
- **THEN** another object or Session SHALL retain its own initial value
- **AND** the assignment SHALL NOT write a database row
