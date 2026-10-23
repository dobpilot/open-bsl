## ADDED Requirements

### Requirement: Acquired configuration root identity access
MetadataCatalog SHALL expose its optional acquired configuration object identity through `configuration_object_id() -> Option<&str>` without substituting the catalog namespace identity.

#### Scenario: Host authorizes a configuration right
- **WHEN** the host reads configuration_object_id after the database adapter supplies the configuration root
- **THEN** it receives the same acquired object identity used by runtime configuration-right checks
- **AND** a catalog without that acquired identity returns None

### Requirement: CLI host-local connection scope
For `bsl-cli --config-db`, the authorized connection snapshot SHALL cover all real configured Session instances of the current CLI process at snapshot creation, including the Session being opened. It SHALL exclude native 1C connections, metadata acquisition connections and VM workers that are not Session instances. The current single-Session CLI SHALL supply its actual connection numbered 1, application name `bsl-cli` and acquired host computer name. Selecting a web-client execution context SHALL NOT identify the executable as a native WebServerExtension.

#### Scenario: Authorized single-Session CLI
- **WHEN** the selected principal's acquired roles grant ActiveUsers and the CLI opens its only configured Session
- **THEN** GetInfoBaseConnections returns that actual host connection with the same number as InfoBaseConnectionNumber
- **AND** its application name is bsl-cli and its computer name is the acquired host name

#### Scenario: Selected principal lacks connection-list access
- **WHEN** the selected principal lacks ActiveUsers or its role rights cannot be established
- **THEN** the CLI does not supply an authorized connection snapshot
- **AND** requesting the list fails explicitly, including during privileged initialization

#### Scenario: Host contains multiple real Sessions
- **WHEN** a host opens a background Session while its parent remains alive and supplies an authorized host-local snapshot
- **THEN** that snapshot includes both actual Session connections with distinct numbers
- **AND** metadata acquisition connections and unrelated native 1C sessions do not appear
