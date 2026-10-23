# Design

## Evidence and boundary
The allocator stack and immutable binary receipt are retained at `docs/research/fixtures/buh-startup-job-runtime-allocation-1016/`. The stack is String growth in write_program_body/write_image called by runtime_for_engine through EngineJobService::is_ready. The earlier active defer-background-runtime-preparation intentionally deferred the expensive operation only until first capability access; this change addresses the measured remaining operation itself.

## Encoding
Bump FORMAT_VERSION44 to45. Within a configuration, an eligible later module or entry uses `.tables shared` in place of both `.names` and `.shapes` sections. It references the first module's two tables, not the previous module. The first module and standalone programs always define complete tables. Different tables retain their complete existing sections. Unknown references and a reference with no first module are parse errors. Parsing preserves shared Arc names and Rc shapes, with the normal image verifier still authoritative for operand bounds. Keep both tables' indices and canonical text round-trip stable.

Writer eligibility requires equal names and equal ordered shape-name IDs. Cheap shared-pointer equality may precede content comparison. Do not add a private worker format, compression, disk spool, thread-unsafe transport or lazy empty-registry behavior. Runtime construction and errors retain their current timing. No external dependency or RT/VM normal dependency changes.

## Validation
A hand-built representation fixture proves that adding modules with the same large tables adds only module bodies, with no timing claim. The complete compiler text round-trip and optimization corpus remain mandatory. Malformed/no-base references and mixed independent tables exercise parser boundaries. Original startup must be rerun; synthetic size checks alone are not startup proof. Existing literal-header tests need separate user approval before editing; prepare their exact mechanical patch after the implementation is reviewable.

## First original run and import suffixes
The first version45 original BUH run still aborts at the same2967994368byte String growth (659.15seconds elapsed, observed RSS9503548KiB). Its27recorded source/binary hashes remained unchanged. Therefore names/shapes-only sharing is not a demonstrated fix for the actual image. Retain this negative evidence in fixture/shared-tables-fix/first-startup.

The existing module_imports/assemble_raw path also copies the ordered visible common-module import set into each program, sometimes preceded by a receiver-specific local import. Extend the same unreleased version45 format with `.imports N shared-tail=K`: read N explicit prefix entries, then append the last K imports of the first configuration module. N is the number of explicit records, not the final table length. Ordinary `.imports N` remains the complete form. The writer selects the longest equal ordered suffix with the first module; aliases, target IDs and global flags must all match. Never rebase on an intermediate module. A reference without a first module, zero/invalid/out-of-range K, extra fields or an oversized explicit count is an error. Keep image verification and all import indices unchanged.

This is a generic representation of existing repeated imports, with no platform-name special case. Add a hand-built reproducer with differing local prefixes and repeated shared suffixes before implementing. Private diagnostic telemetry will report actual table/import/instruction counts before serializing the next real catalog and the completed image size if serialization succeeds. Synthetic proof does not replace that next original run.

## Original run after import suffixes
The immutable BUH run completes worker-image serialization at989211225bytes with1483997184capacity, then exits1 on an unrelated unsupported AccessToken constructor. No allocation abort recurs at the old path under the same12GiBceiling. Actual catalog telemetry records5660programs,5659shared name/shape tables,16150786imports including12076701shared-tail entries,1010991668raw alias bytes and8053438instructions. Observed peakRSS is12276848KiB; this is sampled process RSS, not an isolated serializer-memory or timing benchmark. All28recorded hashes match. Original startup and selected parameter initialization remain unproven. Evidence: fixture/shared-imports-fix/original-startup.


## Retained worker-buffer capacity after successful startup

The original BUH handler now returns, but a separate selected-parameter entry aborts under the unchanged12GiB ceiling. Diagnostic allocation stacks identify duplicate-name verification during component linking, and VmSize is already at the ceiling before this stage. The immutable worker recipe holds989212994text bytes with1483997184capacity; no consumer appends to it or relies on its address.

Before publishing the worker recipe, drop the finished owned temporary BytecodeImage and trim the String capacity. This is internal resource management within the existing representation, not a new BSL/public API/format/transport or eager-cache policy. No verifier check is skipped and no interner algorithm is duplicated. Reuse the existing job tests and required gates; measure exact capacity, pointer movement, startup and selected-stage outcomes under the same ceiling. Shrinking may move the buffer or have allocator-dependent temporary cost; do not promise in-place release or declare the OOM fixed before the replay.

Evidence: `docs/research/fixtures/buh-selected-parameter-allocation-1016/`. The diagnostic run fails on4325392bytes in NameInterner hash-index growth; the baseline fails on3145728bytes. Instrumentation changes allocation layout, so those sizes are not claimed identical. Both runs retain successful original startup and failed selected checks.
