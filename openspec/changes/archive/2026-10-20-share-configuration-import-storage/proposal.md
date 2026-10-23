# Share configuration import storage

## Why
The allocator experiment in docs/research/fixtures/module-import-memory-1020 proves that every importing module retains a separate copy of every visible common-module import. At 800 visible modules this accounts for exactly 51,136,000 additional live bytes. Intermediate resolved programs duplicate the same environment before bytecode assembly.

## What Changes
Use shared immutable ranges for Program.imports, retaining its logical ordered contents. Release intermediate import copies immediately after resolution and construct compact bytecode environments from the common catalog. Preserve the existing textual format and dynamic compiler request contract.

## Impact
The Rust representation of Program.imports changes from Vec to ModuleImports. Existing fixture construction requires mechanical conversions; expected behavior does not change. No platform semantics, dependencies, visibility rules or verifier checks change.
