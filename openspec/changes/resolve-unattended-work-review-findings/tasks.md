# Tasks

## 1. Specification divergences
- [x] 1.1 Replace `a_run_allocates_both_cache_slots_for_every_instruction` with a test that enters a catalog module through the VM and observes its cells; keep an eager-allocation test for a standalone `Program`.
- [x] 1.2 Correct the `attach_catalog`/`RunCaches::for_catalog` comments to describe the per-module boundary.
- [x] 1.3 Add a declared-only flag to builtin and component descriptors, raise one uniform error from every declared-only entry, and print the flag in the generated API reference; regenerate `docs/reference/bsl-api/api.md`.
- [x] 1.4 Implement `ТРег` (and any other declared-only member that needs no new platform semantics) instead of keeping it as a stub.

## 2. Measurement discipline
- [x] 2.1 Add `НЕ ИЗМЕРЕНО` markers, registry entries and `measure-all.bsl` lines (or name the existing probe) for: `ОписаниеОшибки` outside the handler frame and its text format; deferred name errors raised with line 0 versus `RaiseCompilationError`; the `SortDirection` `Ascending` alias; `LegacyFind` of an empty substring; lenient `ПолучитьДвоичныеДанныеИзBase64ДвоичныхДанных`; the empty-date `Формат` case.
- [x] 2.2 Replace every "нативная проба подтверждает"-style comment that names no probe with the probe path or a marker.
- [x] 2.3 Re-measure `tests/conformance/fixtures/structure_duplicate_fields` on the platform and keep the raw platform output beside it; commit the `platform.oracle` receipts behind `chart_plot_area` and `spreadsheet_pattern`.
- [ ] 2.4 Run or delete the 28 `tests/conformance/measure/1c/measure-*.bsl` probes that have no `.platform.txt`; mark hand-assembled native images as such.
- [x] 2.5 Measure the `Отказ` coercion of fill and fill-check handlers once and record it.
- [ ] 2.6 Pin the measurement session environment. Interface language (`Value list`/`Список значений`, `Ascending`/`По возрастанию`, `Да`/`Yes`, and which strings `Булево()` accepts) and script variant (`SortDirection`/`НаправлениеСортировки`) come from the platform `/L` setting and the information base, not from `run-on-1c.sh`; a recreated `/tmp/open-bsl-ib` silently switches them. Committed oracles mix both interface languages (August: Russian, October: English). Decide the canonical environment, set it in the runner, and re-measure the oracles that depend on it.

## 3. Front end, compiler and VM
- [x] 3.1 Replace the constructor-code literals in `bsl-sema/src/core_receivers.rs` with descriptor properties read from the runtime table; remove the added `expect`.
- [ ] 3.2 Make the register-capacity fallback a named plain-pipeline stage that starts from clean tables; compile a configuration catalog once; split `name_methods` from `allow_sparse_large_frame`.
- [ ] 3.3 Run `benchmarks/hot-code-diff.sh` and an alternating A/B for the `Frame`/`step` changes; move the handled-error stack out of `Frame` if the difference is measurable.
- [x] 3.4 Resolve `GetImportedModule` targets through an index fixed at verification.
- [x] 3.5 Move `bind_program_shapes` from `bsl-bytecode` to `bsl-vm`.
- [ ] 3.6 Replace the `include!` copy in `bsl-sema/src/sendable.rs` with a single resolved-tree definition that can cross threads.
- [ ] 3.7 Collapse the `resolve_*_with_*`, `compile_dynamic_snippet_with_*` and `call_configuration_handler_with_*` families into options-struct entry points.
- [ ] 3.8 Remove the per-lexer keyword map layered over the global one, unless an A/B shows it pays; delete `crates/bsl-compiler/examples/name_table_storage.rs`.

## 4. Runtime
- [ ] 4.1 Generate every per-enumeration table in `enums.rs`/`types.rs`/`value/mod.rs` from one declaration; replace the linear lookups by indexed ones; make `kind()` non-panicking.
- [x] 4.2 Merge `call_configuration_instance_handler{,_without_safe_mode,_with_safe_mode}` into one call taking the entry mode; share date/zone parsing between `to_universal_time` and `to_local_time`.

## 5. Configuration and session
- [ ] 5.1 Extract one fill / fill-check handler protocol for catalog, document and business-process objects.
- [ ] 5.2 Replace the `Строка(12)`/`Строка(12,1)` special case in business-process fill checking with general qualifier handling or an explicit unsupported error per type.
- [ ] 5.3 Split `bsl-orm/src/lib.rs` and `bsl-session/src/lib.rs` into modules along existing seams; remove the data-composition stubs from `bsl-orm`.
- [ ] 5.4 Move `run_in_session`, interface-language resolution and host-connection authorization from `bsl-cli` into the library; move the `rustls` provider installation with it.
- [ ] 5.5 Pass startup-timing output as a host option; read `OPEN_BSL_STARTUP_TIMINGS` once in the CLI and list it under `--help` ENVIRONMENT.
- [ ] 5.6 Update the ARCH-02 topology in `docs/architecture/component-architecture.md` for `bsl-config`, the `configuration-files` facade feature and the `bsl-config-db` → `bsl-syntax` edge; decide whether `configuration-files` stays a default feature.
- [ ] 5.7 Record the legal position on decompiling vendor module images in `decompile-native-module-images` before that change resumes.

## 6. Facade and jobs
- [ ] 6.1 Resolve background-job target chunks by method name; share the server common-module filter with `EngineBuilder::configuration_source`.
- [ ] 6.2 Report a failed runtime build from `EngineJobService`; reset `preparing` through a guard.
- [ ] 6.3 Specify `Error::InModule` in the execution-engine specification; drop facade re-exports with no external user (`ExecutionCatalog`, the whole `uuid` module).

## 7. Tests and tooling
- [ ] 7.1 Merge single-probe integration test files into table-driven targets per crate; record `target/` size of one debug test run before and after.
- [ ] 7.2 Replace the per-probe branches of `tests/conformance/measure/1c/run-on-1c.sh` with a setup manifest beside each probe; remove fixed `/tmp` information-base paths.

## 8. Verification
- [ ] 8.1 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.
- [ ] 8.2 `openspec validate resolve-unattended-work-review-findings --strict`; merge deltas and archive.
