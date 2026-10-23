# Tasks

- [x] Record actual session CPU profiles and inspect the ORM/query metadata call paths.
- [x] Strictly validate this change before changing the provider architecture.
- [x] Add collection index regressions covering catalog order, subsystem scope and explicit common attributes.
- [x] Replace repeated collection scans with immutable membership indices.
- [x] Deliver and review the SDK runtime-facts API with full-snapshot equivalence tests.
- [x] Separate provider runtime readiness and query snapshot readiness without duplicating SDK normalization.
- [x] Cover runtime-only growth, subsequent query finalization, reuse, failures and old snapshot identity.
- [x] Measure alternating baseline/candidate startup with serial CLI runs and preserve evidence.
- [x] Pass fmt, Clippy, full workspace tests, rustdoc, facade consumer and dependency checks without unapproved existing-test changes.
- [x] Merge and archive after the complete green run.
