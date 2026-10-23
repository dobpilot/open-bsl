## Tasks
- [x] Measure the reached member, aliases, type names and representation on native 1C.
- [x] Add ChartLabelLocation.EdgeAuto through existing tables and a focused regression.
- [x] Regenerate the API reference and run required checks.
- [x] Replay original BUH and record the next terminal result.
- [x] Add the subsequently measured ChartScaleLabelLocation.None through the same tables.
- [x] Measure the subsequently reached EdgeInside member, aliases, type, presentation and comparison controls.
- [x] Add a red facade regression and extend the existing enum member table with EdgeInside.
- [x] Replay original BUH after EdgeInside and retain its terminal result.

Final verification2026-10-21: full3999/0/1, all8 gates, API-reference freshness and all9 original initialization cases pass. Evidence: `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/test-summary.json` and `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. This closes verification only; declaration-only capabilities remain declaration-only.
