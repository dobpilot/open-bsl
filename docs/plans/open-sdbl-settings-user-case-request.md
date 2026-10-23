# Preserve the exact settings user namespace

## Observed defect

`QueryCompiler::compile_common_settings_load` and
`compile_system_settings_load` currently generate `_UserID = <literal>`.
On the native PostgreSQL Pro 18 schema this column is `mvarchar`: its equality
matches `ControlAdmin` when the caller requests `controladmin`.
The real 1C 8.3.27.2342 platform returns Undefined for the latter namespace.
This affects authorized DataAdministration and privileged reads, not just
permission checking.

Evidence is retained under `docs/research/fixtures/client-settings-rights-1017/`:

- `native/*/platform.txt`: five authenticated profiles, exact user names and
  absent differently cased namespaces;
- `rights/`: native compiled role resources and configuration root identity;
- `replay/result.json`: 95 comparisons through the real CLI and database adapter,
  six mismatches, all differently cased namespace reads by authorized users;
- `replay/key-types.log`: actual settings column types;
- `replay/key-comparison.log`: `ControlAdmin = 'controladmin'` is true for
  `mvarchar`, but false after conversion to `text COLLATE "C"`;
- `owned-settings.dt`: owned native database, with an empty session initializer
  added solely for the adapter replay. This is not production startup evidence.

## Requested change

Make both settings-load compilers preserve the exact supplied user namespace
on the measured PostgreSQL schema. Reuse the existing SQL generation and
literal escaping. Keep explicit empty names literal, omitted-user resolution
in the host, separator scope, bounded result cardinality and read-only behavior.
Do not implement a user lookup, case folding, writes, or another settings reader
in open-bsl. Do not change object/settings key semantics without measurement.

Add regressions for the native `mvarchar` form, canonical and differently cased
names, empty/nonexistent user namespaces, separators and SQL literal escaping.
Replay against the owned native fixture and require all 95 retained read
comparisons to match, including the six presently failing ones.

Classify against the existing OpenSpec exact-user contract before code; add a
change only if the contract needs amendment. Run formatting, Clippy, tests,
rustdoc and strict OpenSpec validation. Document provider coverage honestly.
No new external dependencies. Release/version and agent assignment remain
unassigned pending explicit user authorization.

## Current SDK0.9.25 replay, 2026-10-19

Reproduced all95 original read comparisons with the current verified CLI and unchanged native oracles in a fresh owned restored PostgreSQL database:89 match, the same6 namespace mismatches remain. All five CLI processes exit0, READ ONLY is retained, and the temporary cluster is stopped and removed. Current red evidence: `docs/research/fixtures/client-settings-sdk25-replay-1019/result.json`. No SDK changes were made. Agent/release assignment remains pending explicit authorization.

## Authorization, 2026-10-19

The user assigned agent/release0.9.26 and authorized future agents for open-sdbl follow-up work without repeated launch approval. Agent `open_sdbl_0926_settings_user_case` implements this request in an isolated SDK copy; ROOT reviews/transfers the exact patch and independently replays the retained95 controls. Canonical remains0.9.25 until verified transfer. Earlier pending-authorization statements describe preceding checkpoints.
