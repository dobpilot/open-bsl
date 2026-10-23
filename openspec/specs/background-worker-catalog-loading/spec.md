# background-worker-catalog-loading Specification

## Purpose
Define when background workers construct and reuse their configuration engines so idle pools avoid unnecessary catalog copies while preserving cancellation, shutdown and broken-runtime behavior.

## Requirements

### Requirement: Idle workers defer configuration catalog construction
Background workers SHALL preserve the configured operating-system thread count and bootstrap behavior, but SHALL construct their worker-local configuration engine only when they have claimed queued work. A worker SHALL reuse that engine for subsequent work. Helping execution with an existing engine SHALL continue to use that engine. Existing image checks, eager instruction caches and session isolation SHALL remain in effect.

#### Scenario: Idle worker
- **GIVEN** a worker starts with an empty queue
- **WHEN** it reaches its scheduler wait boundary
- **THEN** no worker configuration engine has been constructed

#### Scenario: Reuse after useful work
- **GIVEN** one worker receives two queued jobs
- **WHEN** both jobs execute on that worker
- **THEN** its configuration engine is constructed once and reused

### Requirement: Deferred construction preserves terminal states
Worker catalog construction SHALL execute outside the job registry lock. Shutdown or cancellation during construction SHALL NOT cause a terminal job to return to Running. Construction failure SHALL preserve existing broken-runtime failure handling and SHALL NOT replace an already Closed runtime with Broken.

#### Scenario: Shutdown during construction
- **GIVEN** a worker has claimed a job and is constructing its first engine
- **WHEN** shutdown completes the job before construction returns
- **THEN** the job remains terminal and its BSL body is not started

#### Scenario: Cancellation during construction
- **GIVEN** a worker has claimed a job and is constructing its first engine
- **WHEN** the job is canceled before construction returns
- **THEN** construction completion does not resurrect or execute that canceled job

#### Scenario: Construction failure
- **GIVEN** the runtime is open and worker engine construction fails
- **WHEN** the failure is processed
- **THEN** the runtime becomes Broken and resident jobs receive the existing failure outcome
