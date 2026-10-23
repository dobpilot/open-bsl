# Tasks

- [x] 1. Validate this change strictly before code.
- [x] 2. Add focused parser and help tests without changing existing expectations.
- [x] 3. Forward the optional exact value through the existing SessionFactory API.
- [x] 4. Run CLI tests, fmt and Clippy; repeat original demo with supplied host registration.
- [x] 5. Archive after required verified gates.

## Verification

The three initial CLI tests failed before implementation. All 61 binary tests now pass (`/tmp/open-bsl-cli-connection-string-tests-1012.log`), including exact-value parsing, rejection before invocation consumption, help and unchanged existing parser tests. Workspace all-targets Clippy passes (`/tmp/open-bsl-current-workspace-clippy-1012.log`), and formatting is clean.

The original read-only demo control uses the same existing SessionFactory builder with an exact registered host string and passes the previous missing-connection-string failure (`/tmp/open-bsl-demo-session-host-connection-1012-startup.log`). It then fails on unsupported exclusive mode; this is not successful Session startup. Main SDK integration and the full workspace test gate remain incomplete, so no archive is performed.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
