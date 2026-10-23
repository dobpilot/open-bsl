# Tasks

- [x] Validate strictly before implementation.
- [x] Locate the original failing caller and retain nineteen native controls.
- [x] Implement coercion and empty-needle handling; add the native conformance fixture.
- [x] Replay controls and existing searches, then repeat original demo.
- [x] Run required checks and archive with the configuration runtime specification.

Native controls replay byte for byte in ordinary and optimized execution. The five existing non-empty search controls also replay unchanged. Original caller is ПодставитьПараметрыВСтрокуИзМассива, module 1294, chunk 4, pc 28.

Final verification (2026-10-21): Native nineteen-case evidence and the five retained search controls are described in `docs/research/str-find-session-startup.md`; `tests/conformance/fixtures/str_find_coercion.{bsl,expected}` participates in the complete plain/optimized conformance corpus. The final workspace log in `docs/research/fixtures/deferred-prepared-module-caches-1021/final-checks-adapted/` records 3999 passed, zero failed, one ignored, including the optimizing-pass corpus check; all eight gates passed. Original DEMO initialization passed for thin-client, web-client and background Session initialization in `docs/research/fixtures/on-demand-session-matrix-deferred-cache-1021/result.json`. Protected export stubs remain the approved default; no protected-body equivalence is claimed.
