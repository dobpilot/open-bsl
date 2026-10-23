## Tasks
- [x] Inspect the reached original event and existing CLI resource wiring.
- [x] Specify the host destination and validate before code.
- [x] Exercise a real BSL event through the CLI writer and verify delivery/failure behavior.
- [x] Attach settings and journal adapters in the isolated CLI and build it.
- [x] Replay original configured CLI initialization and retain its terminal result.
- [x] Integrate and run required workspace gates before archival.

Final authorized integration verification (2026-10-16): ROOT3794passed,0failed,1ignored; SDK1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, consumer build and unchanged RT/VM dependency trees pass. Actual production CLI initializes DEMO successfully. The frozen review matrix covers all nine original session contexts plus selected initialization in all three databases. Evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/` and `docs/plans/configured-session-startup-matrix.json`. Earlier failed/pending outcomes above are historical.
