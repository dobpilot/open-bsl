# Design

## Inline caches: per-module boundary

`RunCaches::for_catalog` creates every module of an indexed catalog as
deferred, and `ready` materialises a module's cells on first entry. The
reason is real — a child session would otherwise pay for every body of a
shared catalog it never enters — so the decision is to keep the behaviour
and move the requirement, not to revert. What must stay true by construction:
no instruction of a module executes before that module's cells exist, a
standalone `Program` run still allocates eagerly, and runs stay isolated.
The test `a_run_allocates_both_cache_slots_for_every_instruction` currently
calls `ready` itself and so proves nothing; it is replaced by one that enters
a module through the VM and observes the cells. The comment claiming that a
full image still creates all caches is false and is corrected.

## Declared-only names

A declared-only entry is a name that exists so that resolution and arity
checks succeed, with no implementation behind it. It is useful — real
configurations compile and only the unreached call fails — but it must not
look like support. Each such entry carries a flag in its descriptor; the
call raises one uniform runtime error naming the function or type; the API
reference generator prints the flag. The flag is the absence of an
implementation (`call: None` on component functions and constructors,
`BuiltinFn::is_declared_only` for core builtins), so a flagged entry cannot
carry a body by construction. Component types are reachable only through
their constructors, so no type needs its own flag; methods of configuration
objects that raise are outside this requirement. Trivial members (`ТРег`) are
implemented instead of being kept as stubs. No platform semantics are
invented: anything implemented later goes through the usual measurement.

## Constructor identity in sema

`core_receivers.rs` compares `ConstructorCode` against literal numbers
(`<= 16 && != 3 || == 27`, `== 6`). The order of constructor registration
becomes a hidden contract. The runtime table gains a per-constructor property
(`uses_core_methods`, `is_background_jobs`) that sema reads; the `expect` the
change introduced goes away with it.

## Plain pipeline and register capacity

On `TooManyRegisters`, `compile_chunk` silently recompiles with
`ssa_regalloc = true`, on name and shape tables polluted by the failed
attempt, and the configuration path can compile the catalog up to three
times. Decision: the fallback is a named, deterministic stage of the plain
pipeline (so `the_optimizing_passes_agree_with_the_plain_run_on_every_script`
compares like with like), it starts from a clean table snapshot, the
configuration path compiles once and reuses the collected names, and the
`name_methods` flag is split from `allow_sparse_large_frame`.

## Hot path

`Frame` grew a `Vec` of handled errors and `step` grew branches for
`ОписаниеОшибки` and `GetCollectionItem`, without `hot-code-diff.sh` or an
alternating A/B. Both are run; if the frame growth shows, the handler stack
moves to the run state keyed by frame depth.

Result (2026-10-23, `instructions:u`, five alternating pairs of release
`bsl-cli` built from `2b9a739` and from this change, loaded machine, so
instructions rather than cycles): `call_overhead` −1.6 %, `pi_leibniz`
−1.9 %, `str_find` −2.9…−3.5 %, `str_index` −7.5 %, `json_parse` −3.5 %,
`regexp` −13.9 %, `goto_bench` +1.5 %; per-pair spread under 0.7 %. The
call-heavy scenario, where a larger `Frame` would show, got cheaper, so the
handler stack stays in `Frame`. The `goto_bench` growth has no frame
traffic and is left as a known cost of the added `step` branches.
`hot-code-diff.sh` against `HEAD` for this change's own VM edit: `step`
identical, `step_cold` 0xa0 bytes smaller.

## Remaining structure

- `GetImportedModule` resolves its target through an index computed during
  verification instead of a linear scan per execution.
- `bind_program_shapes` (runtime binding) moves from `bsl-bytecode` to
  `bsl-vm`; `ExecutionCatalog` stays as the representation-side contract.
- The `SendableResolvedProgram` second `include!` of `resolved_types.rs` is
  replaced by making the number payload of the resolved tree `Send` (or by
  one conversion at the boundary), so there is one definition of the tree.
- The `resolve_*_with_*`, `compile_dynamic_snippet_with_*` and
  `call_configuration_handler_with_*` families collapse into one entry point
  each taking an options struct.
- `enums.rs`: one declaration per enumeration (a `macro_rules!` table that
  generates `EnumKind`, names, members, `ENUM_NAMES`, `TypeId` mapping), and
  lookups by index instead of linear scans; `kind()` stops panicking.
- `bsl-orm`: one handler protocol for `ОбработкаЗаполнения` and
  `ОбработкаПроверкиЗаполнения`; the `Отказ` coercion is measured once and
  applied everywhere. The roots of `bsl-orm` and `bsl-session` are split along
  the seams `catalog_object.rs`/`document_object.rs` already show. The
  data-composition stubs in `bsl-orm` are removed in favour of
  `bsl-rt/src/data_composition.rs`.
- `bsl-cli`: `run_in_session`, interface-language resolution and the
  `ActiveUsers` authorization of host connections move to `bsl-config-db`
  (or `bsl-session`); the CLI parses flags and calls one function. The
  direct `rustls` dependency moves with the code that needs it.
- `open-bsl` jobs: the target chunk is resolved by method name, not by
  declaration position plus one; the module filter shared with the engine is
  one function; `EngineJobService` reports a failed runtime build instead of
  returning empty answers; the `preparing` flag is reset by a guard.
- `Error::InModule` is described in the execution-engine specification.
- Startup timing output is a host option passed in, read from the
  environment once in the CLI.
- Integration tests that are one file per probe are merged into table-driven
  targets per crate: today ~475 test binaries need more than 55 GB of
  `target/` for one debug test run.
- The per-lexer keyword cache stays: lexing every committed `.bsl` file five
  times costs 2.86·10⁹ `instructions:u` with it and 4.23·10⁹ without it
  (+48 %, five alternating pairs, spread under 0.1 %, 2026-10-23). The
  example `crates/bsl-compiler/examples/name_table_storage.rs` stays too:
  `docs/research/unf-compilation-oom-1012.md` cites it as the reproducible
  measurement behind the shared name table.
- `run-on-1c.sh` gets one generic setup mechanism (a manifest beside each
  probe naming input XML and `ChildObjects` additions); the per-probe
  branches and fixed `/tmp` information bases go away.
