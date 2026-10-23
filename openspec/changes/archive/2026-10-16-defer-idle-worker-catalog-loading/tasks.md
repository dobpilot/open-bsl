# Tasks

- [x] Inspect current worker ownership, scheduler and allocation evidence.
- [x] Validate this change strictly before implementation.
- [x] Confirm coordinating-agent authorization: ROOT worker/service are not watched; preserve watched prepare/private/binary files throughout the immutable control.
- [x] Reproduce eager idle construction with a failing new focused test.
- [x] Defer worker engine construction and preserve borrowed helping execution.
- [x] Test one-build reuse and controlled cancellation/shutdown during construction.
- [x] Run focused worker regressions and formatting; report exact outcomes for independent review.
- [x] Complete workspace gates and original read-only startup/selected replay through the coordinating agent.

## Verification outcome

The unfiltered socket-enabled workspace run completed with 3745 passed, 9 unchanged failures and 1 ignored test. Formatting, 125 strict OpenSpec items, Clippy, rustdoc, the consumer, the optimizer corpus and unchanged RT/VM dependency checks pass. Original BUH ThinClient plus selected parameter replay 83119 completed with exit 0 under the same 12 GiB limit with 24 workers; all 59 watched hashes match. The remaining task stays open because the complete workspace is not green; no archive or commit was performed. Evidence: docs/research/fixtures/buh-selected-parameter-allocation-1016/lazy-worker-catalog/.

After the user approved the seven-file existing-test expectation patch, the complete workspace result is 3753 passed, 1 inherited SDK failure and 1 ignored test. All other gates pass. Fresh UNF WebClient plus selected initialization and parent/BackgroundJob startup also pass on the same production binary; approved test changes produce an identical release binary. Archival remains blocked by the unresolved SDK dependency result.

Final authorized integration verification (2026-10-16): ROOT3794passed,0failed,1ignored; SDK1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, consumer build and unchanged RT/VM dependency trees pass. Actual production CLI initializes DEMO successfully. The frozen review matrix covers all nine original session contexts plus selected initialization in all three databases. Evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/` and `docs/plans/configured-session-startup-matrix.json`. Earlier failed/pending outcomes above are historical.
