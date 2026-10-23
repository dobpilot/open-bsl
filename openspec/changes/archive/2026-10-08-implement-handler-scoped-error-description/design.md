# Handler-scoped descriptions

## Evidence and scope
The native 8.3.27.2342 control and hashes are in `docs/research/fixtures/error-description-context/`. Eight observations prove empty descriptions outside handlers, separate ordinary-call scope, and restoration after a nested handler. The control checks explicit-message membership, not byte-for-byte platform stack rendering. The current interpreter runs the same driver and reports `probe.unsupported`.

No conclusion about configuration callbacks, dynamic snippets or asynchronous continuations follows from the ordinary-call control. Measure those paths separately before assigning them platform semantics. Existing error information and rethrow behavior remain authoritative unless an independently measured change explicitly modifies them.

## Existing implementation constraints
`component.rs::error_description` deliberately rejects calls because task-wide `current_exception` cannot represent active-handler scope. `unwind_to_handler` overwrites that task-wide value. `ExceptionRange` stores only protected start/end PCs and the handler entry; it does not store the handler exit. The compiler currently emits an ordinary jump over the handler. The verifier permits independently constructed bytecode and does not require that compiler layout.

Consequently, do not infer the handler end from a neighbouring jump and do not expose task-wide `current_exception` as the implementation. Such shortcuts would silently depend on code layout or preserve the wrong nested/call behavior.

## Representation decision before implementation
Determine the smallest explicit representation of handler lifetime which remains correct across normal fallthrough, return, break, continue, goto, exception unwinding and optimization. Use the existing bytecode/VM pipeline. If this requires changing the serialized representation, amend this change with its versioned format requirement before code, update the single opcode/effects/round-trip tables together, and validate images before use.

Do not modify existing regression expectations to accommodate implementation. If an existing test must change, present the concrete reason and wait for the user's explicit approval under the repository's regression rule. New focused tests and native controls can be added independently.

## Performance and boundaries
No new normal dependency in bsl-rt or bsl-vm, no database access, and no global or thread-local exception storage. Preserve the VM's cold-path discipline; changes that alter the hot dispatch loop require the existing machine-code screen and any performance claim requires alternating A/B measurement. A hot-loop mismatch is evidence to investigate, not a performance result.

## Selected representation
Extend the existing `ExceptionRange` record with an explicit exclusive handler-body end PC and bump the textual format version. Preserve its protected range and entry semantics. Update printing, parsing, verification and every relocation of exception PCs together. This avoids a second parallel handler table and requires no new opcode or opcode-effects classification.

Each VM frame will retain caught error snapshots with their explicit handler body extents. Selecting a description uses only the current frame and the most recent retained handler containing the current PC. Before installing a newly caught error, discard snapshots whose handler does not contain the fault PC; this retains the outer handler during a nested catch and bounds retained snapshots by nesting depth. A popped frame drops all its snapshots.

The source resolver already rejects entering a nested or neighbouring block through goto; legal source control flow cannot leave a handler and jump back into it without a fresh exception. Normal calls have their own frames. This makes extent selection sufficient for the measured source-language paths, including normal fallthrough, return, loop exits and outward goto, without adding checks to every normal VM instruction. This is not an assertion about native bytecode manipulated to enter handler bodies through otherwise impossible source paths.

Move the existing function to the VM-aware builtin path already used for `ИнформацияОбОшибке`; keep its public names and zero arity, and remove only the obsolete component stub registration. Reuse the existing error object when formatting a description. Measure any observable rendering distinction before claiming complete platform text parity.

Production implementation and new focused tests are complete; full regression gates are awaiting approval for five mechanical existing test literals and the existing unsupported-stub expectation in `open-bsl/tests/error_description.rs`. Those changes have been requested explicitly; neither was made before approval.

## Further native controls
`nonlocal-exits.bsl` contributes eight native observations covering return, loop continue/break, outward goto and restoration of an outer handler after a goto out of a nested handler. `dynamic.bsl` contributes eight observations: both dynamic operations see an empty description even while the surrounding call handles an error, and the surrounding handler retains its error afterward. These results support separate dynamic frames rather than propagation of the surrounding active description. Async continuation and configuration callback behavior remain separately unmeasured.

## Presentation evidence and existing diagnostic limit
The four native `presentation.bsl` observations show that `ОписаниеОшибки` differs from the error object's Description property and from both existing brief and detailed presentation functions. None can be claimed to be the native formatter by substitution. `current_error_info` already documents the interpreter's deliberate absence of platform-equivalent module coordinates and call-stack rendering. Preserve that explicit limitation in the initial handler-scoping implementation; the requirements above establish scope, emptiness and message membership, not byte-identical diagnostic rendering. Keep the presentation control as evidence of the remaining format difference, separate from the scope controls intended for exact replay. Do not invent platform coordinates or present generated interpreter output as a native oracle.

The native async control runs in the client form context, as required by the platform. It proves separate parent/child descriptions and preservation across the observed await paths. It does not prove a particular promise was still pending at the instant of awaiting; no suspension-duration or scheduling claim is made. Keep a deterministic VM pending/resume test in addition to the native receipt when implementing.

The new CLI regression file replays the four scope controls (32 native lines) and checks aliases, arity and separate states. Both tests currently fail on the existing deliberate stub; this is the recorded red stage, not a claim that the workspace is green.

## Configuration object control
The native `instance.bsl` plus `ObjectModule.bsl` control adds nine server-side observations on the isolated file infobase. It covers object-body initialization both outside and inside a caller handler, exported method calls, object-local handling with an ordinary nested call, an escaping exported-method exception and restoration in the caller. `bsl-session/tests/error_description_instance.rs` replays those observations through the existing SessionFactory, ORM object instance and VM callbacks. Only the message sink becomes a result array, following the existing session replay convention. The metadata model declares this control object's measured empty attributes and tabular sections; it does not infer missing SDK metadata for production configurations. No additional runtime changes were needed for the replay.

## Release screen result
The repository hot-code screen was run against HEAD 2b9a739. A temporary copy adds only the sibling SDK symlink required by isolated worktree path dependencies; the comparison logic is unchanged. Both step and step_cold differ. This compares all accumulated working-tree changes, not this feature alone, and does not attribute a performance gain or loss. Temporary targets and the worktree were removed. The screen does not clear layout concerns; no alternating A/B or numerical performance claim is recorded.
