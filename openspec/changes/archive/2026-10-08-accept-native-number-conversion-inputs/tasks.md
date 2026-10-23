# Tasks

- [x] Measure source-level numeric conversion on native 1C.
- [x] Validate the change strictly before implementing runtime behavior.
- [x] Add regression coverage for numeric and Boolean inputs and rejected values.
- [x] Reuse the existing VM builtin and keep string parsing unchanged.
- [x] Verify native image recovery against the same receipt.
- [x] Run required regression checks and archive after review.

Verification on 2026-10-08: workspace tests passed (3313 passed, 0 failed, 1 ignored); fmt, all-target Clippy, warning-free rustdoc and facade consumer build passed. Core/VM normal dependency trees match the previous baseline; checked-in API reference and open-question registry checks passed. Logs: `/tmp/open-bsl-workspace-tests-approved-1008.log`, `/tmp/open-bsl-workspace-clippy-approved-1008.log`, `/tmp/open-bsl-workspace-doc-approved-1008.log`, `/tmp/open-bsl-consumer-approved-1008.log`. Existing test updates were explicitly approved by the user before application.
