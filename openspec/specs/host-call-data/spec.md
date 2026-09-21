# host-call-data Specification

## Purpose
Define injection and isolation of session-owned opaque host data in component call contexts.

## Requirements

### Requirement: Session host data capability
A host SHALL supply optional Rc<dyn Any> data when building a State. Component function, constructor, method and property contexts SHALL expose those data. Missing data SHALL return CapabilityMissing with Capability::HostData rather than panic or Undefined. New States SHALL not implicitly inherit the data.

#### Scenario: Read host data from component operations
- **WHEN** a component operation runs in a State with host data
- **THEN** its context exposes that State's supplied value

#### Scenario: Absent data
- **WHEN** a component requests host data in a State without it or a reduced context
- **THEN** it receives CapabilityMissing identifying HostData

### Requirement: Background isolation
Background jobs SHALL NOT inherit the originating State's host data. The API SHALL NOT promise isolation against a host deliberately sharing an Rc or carrying an object between States.

#### Scenario: Foreground data and background call
- **WHEN** a State with host data starts a background job that requests host data
- **THEN** the job observes the missing HostData capability
