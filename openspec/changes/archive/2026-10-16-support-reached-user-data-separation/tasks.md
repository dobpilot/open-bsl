# Tasks

- [x] Measure empty/nonempty properties and authenticated current-user object lifetime.
- [x] Validate strictly before implementation.
- [x] Add the optional Principal snapshot and the lazy user property.
- [x] Test unknown/empty/nonempty data, per-object mutation and separate states.
- [x] Regenerate the reference and run focused fmt/Clippy/doc/tests.
- [x] Integrate the actual SDK acquired data and verify original UNF startup. The SDK0.9.20 review stack passes all three UNF session kinds; the same adapter is now integrated into ROOT after authorization.
- [x] Complete workspace gates and archive.

Final authorized integration verification (2026-10-16): ROOT3794passed,0failed,1ignored; SDK1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, consumer build and unchanged RT/VM dependency trees pass. Actual production CLI initializes DEMO successfully. The frozen review matrix covers all nine original session contexts plus selected initialization in all three databases. Evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/` and `docs/plans/configured-session-startup-matrix.json`. Earlier failed/pending outcomes above are historical.
