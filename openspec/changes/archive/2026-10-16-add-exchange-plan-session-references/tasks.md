# Tasks

- [x] Read and retain the full vendor EmptyRef and CreateNode pages.
- [x] Validate this change strictly before implementation.
- [x] Measure the empty-reference controls on the real platform in an isolated owned filebase.
- [x] Add the reached manager descriptors and reuse the existing reference implementation; test native controls, receiver binding and explicit CreateNode failure.
- [x] Regenerate the API reference and repeat original DEMO initialization.
- [x] Complete regression gates and archive only after required checks pass.

- [x] Read full vendor ThisNode/GetRef pages after the original manager reaches them; retain original call sites.
- [x] Measure eleven GetRef controls, reuse the existing constructor and verify all25native output lines plus scoped unsupported ThisNode delivery.

The six focused tests pass. Frozen ROOT gates report3769passed/1inherited SDK0.9.15 GUID failure/1ignored; other gates pass. All nine original startup cases (DEMO, BUH and UNF across ThinClient, WebClient and BackgroundJob), plus selected initialization in all three databases, pass on the SDK0.9.20 review stack with exact frozen hashes. Production SDK and database-adapter integration remain pending; these replay results do not certify the current ROOT dependency checkout.

Final authorized integration verification (2026-10-16): ROOT3794passed,0failed,1ignored; SDK1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, consumer build and unchanged RT/VM dependency trees pass. Actual production CLI initializes DEMO successfully. The frozen review matrix covers all nine original session contexts plus selected initialization in all three databases. Evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/` and `docs/plans/configured-session-startup-matrix.json`. Earlier failed/pending outcomes above are historical.
