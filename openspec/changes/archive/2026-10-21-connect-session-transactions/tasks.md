# Tasks

- [x] 1. Retain native lifecycle and data rollback controls from own 1C databases.
- [x] 2. Validate this change strictly before implementation.
- [x] 3. Add focused lifecycle and read-denial tests before adapter integration.
- [x] 4. Connect the existing adapter to prepared SDK primitives on its owned connection.
- [x] 5. Verify SDK errors, Session isolation and measured nesting with unchanged existing expectations.
- [x] 6. Repeat original demo and retain its actual startup outcome.
- [x] 7. Integrate approved SDK, complete workspace gates and archive after verified completion.

## Historical isolated implementation evidence

The following records describe the pre-integration state on October 12; they are superseded by the final verification below.

Adapter patch: `/tmp/open-bsl-session-transaction-adapter-1012.patch`; applies cleanly, not yet applied to the main SDK consumer. Existing API report integration and SDK commit32f78d7 are combined only in `/tmp/open-sdbl-startup-combined-1012`.

- Before implementation, both new BSL/live-PG tests fail with the original unsupported transaction diagnostic (`/tmp/open-bsl-session-adapter-repro-1012.log`).
- Final configured own-PG regression: 469 unit tests and one integration test pass (`/tmp/open-bsl-session-adapter-final-regression-1012.log`), including same-connection identity, cancelled-read isolation, repeated begin, SQL failure cleanup and closed-session refusal.
- Final all-targets Clippy passes (`/tmp/open-bsl-session-adapter-final-clippy-1012.log`).
- The first original demo repeat failed through a diagnostic tracing wrapper that did not forward transaction methods. The equivalent UNF repeat was stopped as invalid. All eleven implemented adapter methods are now forwarded; the corrected demo repeat is running. These invalid repeats do not disprove the adapter implementation or prove startup success.

## Final integrated verification (2026-10-21)

`crates/bsl-config-db/src/data_access.rs` delegates begin, commit, rollback and active-state checks to its owned `BlockingDatabaseSession`. Logical depth and cancellation remain per adapter; outer begin calls `begin_read_only_transaction`, and cancelled outer completion calls SDK rollback.

Real-connection verification is retained in `docs/research/fixtures/local-postgres-session-adapter-1016/transactions/`: all five tests passed with the explicit PostgreSQL environment enabled on an isolated disposable cluster. The test checks backend identity, nested transaction lifetime, read denial after inner cancellation, independent Session state, SQL-error cleanup and closed-connection failure. This evidence is separate from environment-gated workspace tests.

Original DEMO initialization passed in all three session kinds in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Final integrated SDK is 0.9.35; workspace verification in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/` records 3999 passed, zero failed, one ignored and all eight gates green. Protected-body execution remains explicitly deferred.
