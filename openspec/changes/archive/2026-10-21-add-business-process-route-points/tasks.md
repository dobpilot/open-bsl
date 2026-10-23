# Tasks

- [x] Validate this change strictly before code.
- [x] Record native collection, named-reference and identity behavior on the own control.
- [x] Project SDK composition and generated type while preserving unknown data.
- [x] Expose qualified and own-manager property access through the existing facade.
- [x] Test owner-scoped identity, known empty and unknown composition, aliases and variable precedence.
- [x] Replay measured runtime behavior and verify native SDK projection.
- [x] Regenerate the API reference and run workspace gates and dependency-tree checks.
- [x] Repeat read-only original demo compilation and record its actual outcome.
- [x] Archive only after implementation and required checks pass.

Final verification (2026-10-21): `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/` records all eight green gates, API generator freshness, consumer closure and unchanged core dependency trees. Both native SDK route-point projection and ten-point facade replay passed. Original DEMO initialization also passed in thin-client and web-client cases under `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`; default protected export stubs remain in effect. This does not claim protected-body equivalence.
