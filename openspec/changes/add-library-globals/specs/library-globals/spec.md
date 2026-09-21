## ADDED Requirements

### Requirement: Library global resolution
A registered library SHALL declare global names backed by a value-returning function in the same library accepting zero arguments. Every read SHALL invoke that function. Names SHALL compare case-insensitively using the runtime folding rule. Local and module variables SHALL retain their existing priority. The only replaceable built-in global SHALL be Metadata; without registration its existing implementation SHALL remain unchanged.

#### Scenario: Read and shadow a global
- **WHEN** a library global is read alone or as the base of property access
- **THEN** its backing function is invoked unless a local or module variable shadows it
- **AND** repeated reads invoke the function repeatedly

#### Scenario: Replace standalone metadata
- **WHEN** a library declares Metadata
- **THEN** reads use that library in either language alias declared by the library
- **AND** without that library the standalone metadata stub is unchanged

### Requirement: Validate global declarations
Engine construction SHALL reject duplicate global names or aliases, reserved global names other than Metadata, and backing functions that are absent, do not return a synchronous value, or cannot accept zero arguments. Undeclared identifiers SHALL retain their existing diagnostic.

#### Scenario: Invalid composition
- **WHEN** two libraries declare the same folded global name or a descriptor has an invalid target
- **THEN** engine construction fails before compilation or execution

#### Scenario: Unknown identifier
- **WHEN** an undeclared identifier is compiled
- **THEN** the existing UndefinedVariable diagnostic and text are preserved
