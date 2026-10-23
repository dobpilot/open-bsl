## ADDED Requirements

### Requirement: Acquired scheduled-job method name
Database-backed configuration metadata SHALL expose the method name of a scheduled job when its explicit module/procedure link has been acquired in the measured form. Russian ИмяМетода and English MethodName SHALL return the same read-only string consisting of the common-module name and procedure name separated by a dot.

#### Scenario: Native method link
- **WHEN** a scheduled job explicitly references a common module and procedure in the supported acquired form
- **THEN** both aliases return the qualified method name without the XML CommonModule prefix
- **AND** reading the property does not execute the job

#### Scenario: Invalid explicit target
- **WHEN** a recognized link targets a missing resource or a resource that is not the referenced common module
- **THEN** acquisition fails without substituting an inferred module name

#### Scenario: Unknown serialized form
- **WHEN** a scheduled-job description does not match a measured supported form
- **THEN** projection does not fabricate a method-name value
