## ADDED Requirements

### Requirement: Opt-in startup phase timings
When OPEN_BSL_STARTUP_TIMINGS equals 1, configured CLI startup and its compiler preparation SHALL emit stderr records containing the prefix open-bsl.startup, a stage identifier and elapsed seconds separated by tabs. Parent stage durations SHALL include their nested stages. Records SHALL contain no credentials, source text or SQL. Without this setting existing stdout and stderr behavior SHALL remain unchanged.

#### Scenario: Diagnose a successful configured startup
- **WHEN** a configured CLI startup completes with timing diagnostics enabled
- **THEN** acquisition, factory creation, session opening and entry execution have separate elapsed-time records
- **AND** compiler preparation records distinguish dependency selection, code generation and image verification

#### Scenario: Preserve ordinary output
- **WHEN** timing diagnostics are unset or have a value other than 1
- **THEN** no startup timing records are emitted

### Requirement: Preserve rooted configuration compilation
Reusing dependency-discovery analysis SHALL preserve the selected module set, compact deterministic module numbering, bytecode format, import visibility, initialization ordering, error behavior and execution results. Unselected module bodies SHALL NOT gain semantic validation or retained syntax solely for this reuse. Selected programs SHALL retain shared import backing storage.

#### Scenario: Remove modules preceding a dependency
- **WHEN** unreachable modules precede or separate selected modules
- **THEN** variable links, function links and module-valued expressions refer to the same selected targets under compact numbering
- **AND** the resulting catalog matches compilation of the corresponding explicitly pruned recipe

#### Scenario: Keep component-property discovery separate from runtime access
- **WHEN** a host component-property mapping makes a manager module reachable
- **THEN** that module remains selected
- **AND** the compiled field access still follows the normal component path without synthetic empty-alias imports
