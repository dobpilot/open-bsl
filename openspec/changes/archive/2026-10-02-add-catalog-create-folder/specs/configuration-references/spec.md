## ADDED Requirements

### Requirement: Acquired catalog hierarchy kind
The configuration catalog SHALL preserve the acquired optional hierarchy kind separately from its hierarchy flag. Unknown kind or flag SHALL fail explicitly when folder creation requires them and SHALL NOT alter existing CreateItem behavior.

#### Scenario: Unknown folder settings
- **WHEN** folder creation is requested without known hierarchy settings
- **THEN** creation SHALL fail explicitly without fabricating a folder
- **AND** existing CreateItem behavior SHALL remain available

### Requirement: Native catalog folder factory
In the absence of an existing same-name manager export, catalog managers SHALL expose zero-argument СоздатьГруппу/CreateFolder through qualified calls and the existing own-manager receiver context. For folders-and-items kind, the returned object's IsFolder SHALL follow the acquired hierarchy flag. For item-only kind, the operation SHALL fail with either hierarchy flag. New-object memory SHALL remain isolated and creation SHALL NOT persist a row. Previously callable manager exports SHALL retain their behavior and argument count.

#### Scenario: Hierarchical folders-and-items
- **WHEN** CreateFolder is called for a hierarchical folders-and-items catalog
- **THEN** its module body SHALL observe IsFolder false and reject assigning it
- **AND** the selected returned object SHALL observe IsFolder true, read-only through both aliases

#### Scenario: Disabled hierarchy
- **WHEN** CreateFolder is called with hierarchy disabled and folders-and-items kind
- **THEN** the module body and returned object SHALL observe IsFolder false

#### Scenario: Item-only kind
- **WHEN** CreateFolder is called with item-only kind and either hierarchy flag
- **THEN** the operation SHALL fail before constructing the object

#### Scenario: Existing item factory and argument count
- **WHEN** CreateItem is called with the same metadata
- **THEN** it SHALL retain its prior non-folder behavior
- **WHEN** CreateFolder receives an argument
- **THEN** the ordinary component arity error SHALL be returned

#### Scenario: Existing manager export
- **WHEN** a manager already exports CreateFolder with one argument
- **THEN** the export SHALL remain callable with that argument and retain its result
- **AND** the newly added native factory descriptor SHALL NOT replace it
