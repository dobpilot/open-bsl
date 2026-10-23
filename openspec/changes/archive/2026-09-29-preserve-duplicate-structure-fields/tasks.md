## Implementation

- [x] 1. Record the literal, computed, missing-value, iteration-order and repeated-Insert platform results in a conformance script and checked-in oracle.
- [x] 2. Route only duplicate literal field lists through the existing runtime Structure constructor; keep unique literals shaped.
- [x] 3. Preserve the last spelling of a replaced Structure field in iteration, fixed copies and serialized values without changing folded lookup.
- [x] 4. Add focused tests for constructor values, evaluation, field order, spelling and bytecode verification.
- [x] 5. Run formatting, Clippy, workspace tests, rustdoc, dependency-tree checks and strict OpenSpec validation; archive the change after all gates pass.
