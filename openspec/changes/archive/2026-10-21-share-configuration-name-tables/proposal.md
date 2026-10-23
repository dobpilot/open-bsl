# Share configuration name tables

## Why
The original UNF build was killed by the kernel at about 20 GiB anonymous RSS before SessionFactory construction returned. Configuration compilation deep-copies the final immutable name table into every module. A retained 64-module diagnostic proves 64 separate allocations with equal contents and 4,788,224 bytes of counted string/table payload. This is one verified duplication source, not a complete memory profile.

## What Changes
- Store Program.names as a shared immutable `Arc<[String]>` and freeze a compiled catalog's final names once.
- Share equal module name tables when reading configuration images while preserving verification and serialized bytes.
- Preserve private mutable name interning for entry/dynamic compilation and existing NameId meanings.
- Adapt callers and prepare any mechanical existing-test edits as a reviewable patch before applying edits that require user approval.

## Impact
Public Rust representation changes in bsl-bytecode and its facade consumers. No BSL behavior, opcode, format version or external dependency changes. Retained AST/resolved data and shapes are separate potential memory costs; successful original startup is still required.
