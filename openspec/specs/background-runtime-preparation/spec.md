# background-runtime-preparation Specification

## Purpose
Specify deferred preparation of the shared background runtime while preserving capability diagnostics, host service compatibility and per-state isolation.

## Requirements

### Requirement: Defer unused background preparation
Creating or running a configuration state SHALL NOT prepare the background worker image or runtime before background capability access. Explicit Engine::job_runtime access SHALL continue to prepare and return the existing shared runtime.

#### Scenario: Ordinary session initialization
- **WHEN** a configuration State is constructed and its handler executes without requesting background services
- **THEN** no background worker image or runtime is prepared

#### Scenario: First background capability request
- **WHEN** a state first requests background services
- **THEN** the existing runtime is prepared once and shared by states of that Engine
- **AND** caller session tokens and host profiles remain isolated as before

### Requirement: Preserve capability and service behavior
External BackgroundJobService implementations SHALL remain ready by default. The service trait and its wait-result type SHALL be reachable through the open-bsl facade. A failed facade runtime preparation SHALL produce the existing CapabilityMissing diagnostic on CallContext access. Submission, snapshots, waits, cancellation and worker execution SHALL retain their existing contracts.

#### Scenario: Preparation fails
- **WHEN** the configured engine cannot prepare its background runtime
- **THEN** CallContext background access fails with CapabilityMissing for BackgroundJobs and the original call path

#### Scenario: Existing host service
- **WHEN** a host supplies an existing BackgroundJobService implementation
- **THEN** the default readiness hook permits its unchanged operations
