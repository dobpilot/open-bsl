# Design

Store import tables as ordered ranges into Arc-backed immutable arrays. A configuration creates one common array; individual environments share ranges around their omitted self import and retain explicit aliases separately. Equality, iteration and indexing observe the original sequence. Cloning preserves shared backing storage. This avoids quadratic retained arrays as well as duplicated alias strings; merely interning strings would retain quadratic entries.

Immediately replace each resolved module's import vector with an empty vector after extracting its compact bytecode environment. Semantic resolution remains unchanged, including explicit alias conflicts and self handling. Assign compact environments after configuration assembly. One resolver invocation can still allocate an O(M) temporary surface; this change does not promise to remove all allocation or all quadratic compilation work.

The current text format remains unchanged. Its existing shared-tail references should retain shared ranges when parsed, rather than copying their contents. Explicit prefix entries also reuse matching records from the first module, so omitted self imports do not force copying the prefix. Complete explicit sections remain accepted. Dynamic requests continue receiving slices: materialize the calling module's environment temporarily when compiling a dynamic fragment, without retaining that materialization in the compiled program. VM/RT dependencies and verification remain unchanged.

This is an internal representation correction with a public Rust field-type adjustment, not a decision about 1C semantics. No platform measurement marker is required. New memory evidence must use separate artifacts and alternate the frozen baseline and fixed probe, never overwrite the original experiment. No original large configuration startup is required or authorized by this change.

## Results

The final 40-process alternating comparison retains all 639,200 logical imports at 800 visible modules while reducing live requested Rust bytes from 52,453,943 to 1,490,759 and the construction peak from 111,312,927 to 3,453,871. All allocations return to baseline after dropping the engine and image. Final production source hashes match the recorded build inputs. This is not a claim about the complete original BUH startup or execution speed.

Workspace validation passes: 3,939 tests, zero failures, one intentionally ignored test; formatting, all-target Clippy, rustdoc with warnings denied and the facade consumer build pass. RT and VM normal dependency trees match the previous baseline byte for byte. Existing fixture constructors were converted only after explicit user approval; assertions and expected results remain unchanged. Evidence is in docs/research/fixtures/shared-import-storage-fix-1020/.
