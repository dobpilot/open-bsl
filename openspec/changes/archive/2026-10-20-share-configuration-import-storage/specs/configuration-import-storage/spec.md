## ADDED Requirements

### Requirement: Shared ordered import environments
Program.imports SHALL represent its ordered imports with shared immutable backing storage. An Engine built from N modules with M implicitly visible common modules SHALL retain O(N + M) import storage when there are no explicit aliases, rather than N copies of M imports. The intermediate resolved collection SHALL not retain those copies simultaneously.

#### Scenario: Common modules omit their own import
- **WHEN** multiple named modules are compiled
- **THEN** their environments share the common backing storage
- **AND** each environment excludes its own module without changing the order of other imports

#### Scenario: Explicit aliases and global exports
- **WHEN** explicit aliases and global common modules coexist
- **THEN** static and nested dynamic calls observe the same aliases, global flags and visibility as before

### Requirement: Preserve import image and dynamic contracts
Textual bytecode encoding, rejection of invalid images, dynamic request slice contents, and name resolution priorities SHALL remain unchanged. Decoding an existing shared-tail section SHALL retain shared backing storage rather than copy its imports.

#### Scenario: Decode and reprint a shared tail
- **WHEN** an image containing shared-tail import sections is decoded and printed
- **THEN** its logical imports and canonical text are unchanged
- **AND** the referenced tail shares storage with the first module

#### Scenario: Missing dynamic name remains missing
- **WHEN** a dynamic fragment references a module outside its caller's import environment
- **THEN** it retains the existing failure rather than gaining catalog-wide visibility
