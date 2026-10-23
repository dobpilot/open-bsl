# Tasks

- [x] 1. Inspect original UNF calls and obtain the full native syntax-help signature.
- [x] 2. Strictly validate before implementation.
- [x] 3. Add the method declaration and explicit failure, retaining existing exports.
- [x] 4. Verify aliases, zero to two arguments, rejection of three arguments, unchanged object state and retained exports. `document_write_boundary` covers both aliases, all declared arities, number/reference/newness, a custom attribute, a populated tabular section, an uncalled BeforeWrite handler and previously callable exports (2 tests, 2026-10-07).
- [x] 5. Regenerate the API reference and repeat original UNF startup.
- [x] 6. Run workspace gates, consumer, dependency trees and strict validation.
- [x] 7. Archive after verification; retain persistence as unfinished work.

Verification on 2026-10-08: workspace tests passed (3313 passed, 0 failed, 1 ignored); fmt, all-target Clippy, warning-free rustdoc and facade consumer build passed. Core/VM normal dependency trees match the previous baseline; checked-in API reference and open-question registry checks passed. Logs: `/tmp/open-bsl-workspace-tests-approved-1008.log`, `/tmp/open-bsl-workspace-clippy-approved-1008.log`, `/tmp/open-bsl-workspace-doc-approved-1008.log`, `/tmp/open-bsl-consumer-approved-1008.log`. Existing test updates were explicitly approved by the user before application.
