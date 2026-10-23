# Tasks

- [x] Inspect the original helper without executing configuration BSL and save native constructor/body/text and exact-output controls.
- [x] Measure initial document structure, title initialization, body replacement and HTML text/empty-element output needed for implementation (native structure receipt covers initial html/body, replacement, title, escaping, br and div; other forms remain unmeasured).
- [x] Register Russian and English document/writer names with measured arities through the existing component mechanism.
- [x] Implement HTML document construction and body/text operations using the existing DOM ownership and identity model.
- [x] Implement string-backed HTML output through existing DOM writer traversal without changing XML mode.
- [x] Replay saved native controls and add focused regression tests for unchanged XML output and invalid usage.
- [x] Repeat original demo compile-only and report the next barrier separately from successful startup.
- [x] Regenerate the API reference and pass fmt, workspace Clippy/tests, rustdoc, consumer and dependency-tree checks; stop before changing conflicting existing tests.
- [x] Merge and archive the OpenSpec delta after the required checks pass.

Final reconciliation (2026-10-21): exact native DOM tree/type/string controls and registered writer output pass in `html_dom.rs` and `html_writer_output.rs`; runtime HTML lifecycle, identity and retained XML behavior tests also pass in `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/workspace-tests.log`. The complete workspace and eight checks pass (4006/0/1), including API freshness and facade consumer. The old HTMLWriter negative constructor expectation was among the previously user-approved two expectation patches; it is not newly changed during this audit. Original DEMO compilation and startup are superseded by successful initialization in all three session kinds in the retained SDK0.9.35 matrix and the current SDK0.9.36 thin/web receipts. Background refresh is still running separately. Unsupported HTML parsing remains explicit, never substituted with XML parsing.
