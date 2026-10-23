# Preserve field names across explicit Session instances

## Why
A retained one-State reproducer creates a Structure in entry A and passes it to entry B. The value reads correctly inside A, but adding an unrelated field access to A changes its local NameId order and makes B report UnknownField. The same defect prevents a host from reading verification-journal Structure.Right after passing that value to another explicit instance. Matching name pools merely hide it.

## What Changes
- Preserve field spelling, mutable identity and aliases when values pass among explicit module instances in one State.
- Retain runtime-created names across sequential calls and ordinary BSL failures.
- Keep separate States and legacy independent run/start behavior isolated.
- Reuse the existing runtime name/shape machinery and cold module-binding boundary; do not serialize/copy the live value graph or add process-global runtime names.

## Impact
No new BSL syntax, database functionality or dependencies. Internal ownership and any necessary VM/facade Rust plumbing are specified before implementation. Existing public instance APIs remain the consumer interface. Immutable compiled modules/images, shared catalog tables and dependency boundaries must be preserved. No performance claim is made.

Evidence: docs/research/fixtures/document-owner-read-right-1019/structure-cross-module-result.json and the journal isolation receipts. The overarching configuration-session change remains incomplete.
