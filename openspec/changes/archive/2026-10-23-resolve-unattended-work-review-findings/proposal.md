# Why

About a month of unattended agent work (2026-09-25 … 2026-10-22, 142 archived
changes) was reviewed before being committed. The small, unambiguous defects
were fixed in place; the review also found architectural debt and two
divergences from accepted specifications that need deliberate decisions
rather than drive-by edits. This change collects them so that each one is
resolved, measured or explicitly accepted instead of being forgotten.

# What Changes

- **Inline caches.** The VM now allocates a module's inline-cache cells on
  the first entry into that module of a catalog run, while the accepted
  `bytecode-image` requirement still demands cells for every instruction of
  every chunk before the first instruction. The requirement is modified to
  the per-module allocation boundary, and a test pins the new boundary
  instead of the one that was bent to pass.
- **Declared-only names.** About seventy global functions, constructors and
  component types (`ТРег`, `ЧислоПрописью`, `Шрифт`, `Цвет`, `ДеревоЗначений`,
  `WSПрокси`, `ЗаписьPDF`, …) are registered only so that vendor modules
  compile, and always raise at run time. The generated API reference lists
  them as supported. They become an explicit, uniformly reported category
  that the reference marks.
- **Internal architecture** (no observable change; tracked as tasks):
  constructor identity in sema by magic numbers; silent register-allocator
  recompilation in the plain pipeline; the `include!`-based sendable copy of
  the sema tree; per-execution linear import lookup; runtime binding code in
  the representation crate; API sprawl of `*_with_*` wrappers; a
  nine-place-per-enumeration layout of `enums.rs` with linear lookups;
  duplicated object fill/check handlers whose `Отказ` semantics already
  diverged; monolithic `bsl-orm`/`bsl-session` roots; session-opening
  orchestration living in `bsl-cli`; startup timing switches read from the
  environment inside libraries; hundreds of single-test integration binaries;
  and a measurement runner that grew one hard-coded branch per probe.
- **Measurement discipline.** Behaviour decided without a platform run gets
  the three-part `НЕ ИЗМЕРЕНО` marker or a named probe, and oracles without a
  platform receipt are re-measured.

# Scope

In scope: the findings listed in `tasks.md`. Out of scope: new platform
features, and the two open changes `decompile-native-module-images` and
`load-file-configuration-with-database-orm`, which keep their own task lists
(the second still has unchecked verification and provenance items).

# Impact

Affected crates: `bsl-sema`, `bsl-compiler`, `bsl-bytecode`, `bsl-vm`,
`bsl-rt`, `bsl-orm`, `bsl-session`, `bsl-config-db`, `bsl-cli`, `open-bsl`.
Affected specs: `bytecode-image`, `bsl-api-reference`. No new external
dependency.
