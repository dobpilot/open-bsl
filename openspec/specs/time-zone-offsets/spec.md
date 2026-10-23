# time-zone-offsets Specification

## Purpose
Specify measured timezone offset functions through the host timezone services, preserving aliases, optional arguments and explicit invalid-input failures.

## Requirements

### Requirement: Daylight time offset
The runtime SHALL expose СмещениеЛетнегоВремени and DaylightTimeOffset as aliases with zero through two arguments. It SHALL return numeric daylight adjustment in seconds for the selected zone and UTC instant. An omitted or Undefined zone SHALL use the computer-zone service, and an omitted or Undefined instant SHALL use the host clock.

#### Scenario: Fixed zone
- **WHEN** the caller requests GMT+3 or GMT-05:30 for a valid instant
- **THEN** the daylight adjustment SHALL be zero

#### Scenario: Regional daylight saving
- **WHEN** the caller requests America/New_York or Europe/Berlin for noon UTC on 15 January and 15 July 2024
- **THEN** the adjustment SHALL be zero in January and 3600 in July

#### Scenario: Measured conversions
- **WHEN** the instant is a valid date string accepted by the native oracle
- **THEN** the runtime SHALL use the corresponding UTC instant
- **AND** malformed date strings, numeric or array instants and non-string defined zone arguments SHALL fail

#### Scenario: Invalid name and arity
- **WHEN** the caller supplies an unknown zone or more than two arguments
- **THEN** the call SHALL fail instead of producing an offset
