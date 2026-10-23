## Specification
- [x] Inspect the current For Each lowering and object protocol, and retain the native non-indexable collection receipt.
- [x] Validate this change strictly before implementation.

## Implementation and verification
- [x] Add a failing external host test for iteration with rejected public indexing.
- [x] Add the defaulted protocol operation and wire existing compiler/VM iteration through it.
- [x] Extend instruction effects, bytecode checks, printer/parser and round-trip coverage; update the format version.
- [x] Prepare the exact format-expectation patch and obtain the required approval before applying it to existing tests.
- [x] Verify existing indexed collection behavior, loop control, optimization parity and facade closure.
- [x] Run workspace fmt, Clippy, tests and rustdoc; inspect RT/VM normal dependency trees.
- [x] Merge the delta and archive only after the complete change passes its gates.
