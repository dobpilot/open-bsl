# Tasks

> Status: deferred by explicit user decision on 2026-10-21. Full execution
> without stubs is outside the current session-initialization goal and does
> not block its completion. Preserve implementation, fixtures and unfinished
> tasks; resume this work only on a separate request.

- [x] Inspect the real image-only buh module using the supplied format references.
- [x] Validate this change with OpenSpec strict validation before implementation.
- [x] Author a control module and record its execution receipt on native 1C.
- [x] Verify the control source through the existing open-bsl facade against that receipt.
- [x] Verify the control configuration and module identity across both CF container segments, compare their source with the authored and re-exported source after BOM/newline normalization, and distinguish a text-only artifact from a compiled-image fixture.
- [x] Acquire the corresponding native compiled image, verify its configuration and module identity, verify its declaration and instruction tables, and compare its native execution receipt with the authored source receipt.
- [x] Verify raw buh tables through the existing serialized parser, preserving omitted sections and rejecting malformed records and header tokens.
- [x] Execute a self-contained hand-assembled constant-return image on native 1C and recover equivalent BSL through the facade, without treating it as the full compiler fixture.
- [x] Measure hand-assembled reference/value parameter images and recover their assignment/return bodies through the facade against separate native receipts.
- [x] Add failing tests for image-only loading and malformed or unsupported images.
- [x] Reuse the existing container and serialized parsers to decode native module images.
- [x] Measure native acceptance of SCSU using the independently compiled control image re-encoded by ICU.
- [x] Decode signed SCSU images without new dependencies, verify all standard tag forms and reject malformed encoding.
- [x] Measure higher/lower numeric table headers on native 1C without inferring their purpose.
- [x] Preserve actual table records independently of their numeric header and verify the full native receipts.
- [x] Validate measured unused root/method label tables and preserve the native loop receipt.
- [x] Measure private module-variable initialization and independent object storage using source and a hand-assembled image on native 1C.
- [x] Recover the measured initializer through the existing BSL pipeline, keeping ordinary body boundaries and object storage intact.
- [x] Recover measured supported bodies without source-line instructions and reject malformed exits.
- [x] Recover measured true-predicate statement/expression jumps and reject corrupt targets.
- [x] Normalize native-proven unreachable pure-constant/forward-jump regions while protecting all entries and incoming edges.
- [x] Recover measured method-local forward label jumps and structured back edges, validating missing references and target boundaries.
- [x] Normalize measured constant boolean prefixes and revisit guards after removing impossible edges.
- [x] Protect executable incoming paths using conservative reachability, including mutually referencing dead regions.
- [x] Recover measured explicit boolean conversion through the existing builtin, including native types, errors and reference assignment.
- [ ] Recover measured asynchronous method declarations and verify promise types, references, visibility and rejected boundaries.
- [x] Recover the independently measured private parameterless async constant-return function, preserving the native promise type and rejecting malformed bodies, entries and parameter tables.
- [x] Preserve measured next-command jumps with values and reference targets in expression state.
- [x] Preserve measured opcode-32 short-circuit boundaries and right-operand reference effects.
- [x] Normalize measured pure constant short-circuit predicates while protecting entries and executable incoming paths.
- [x] Recover the measured collection-loop form without its check line marker and with one back-edge trampoline, verifying all native receipts and malformed boundaries.
- [x] Preserve measured renamed iterator storage while rejecting invalid flags, slots and cleanup.
- [x] Recover measured native numeric conversion through the existing builtin and verify errors, reference effects and malformed operands.
- [x] Recover measured compact conditional collection-loop breaks and reject unvalidated cleanup targets.
- [x] Recover measured procedure early exits directly to the terminal command and reject foreign or nonterminal targets.
- [x] Recover measured compact conditional procedure returns without discarding unknown tails.
- [x] Recover measured unreachable structural joins after procedure return while protecting executable incoming paths and other entries.
- [x] Normalize measured single-jump branch and backward-edge targets within the current method without removing executable trampoline entries.
- [x] Recover measured terminal iteration branches while preserving break and structural back-edge commands.
- [x] Recover measured known unreachable conditional prefixes after validated procedure exits without omitting unknown tails or executable entries.
- [x] Recover measured shared procedure blocks as ordinary BSL labels through the existing facade and reject unsupported block boundaries.
- [x] Recover measured sealed empty handlers in shared-block procedures and reject interior entries or malformed boundaries.
- [ ] Reconstruct supported structured BSL, preserving references and evaluation order.
- [x] Reconstruct the complete independent seven-method compiler fixture and compare all ten execution receipt lines through the facade, including defaults, reference updates, branches, loops, short-circuit side effects and caught exceptions.
- [x] Load the complete compiler fixture through the existing Module.bin resource reader and verify execution and private visibility; reject corrupt final-method handlers without returning a partial module.
- [x] Integrate reconstructed source into the existing selected code snapshot path.
- [ ] Compare fixture execution through open-bsl with native receipts.
- [x] Measure and preserve scalar operator and the eight measured builtin results before reference-mutating calls.
- [x] Measure result capture for the remaining supported builtin commands before later operand mutation.
- [ ] Extend the independently compiled control fixture to cover the additional multi-argument calls, constructors and repeated context calls, distinguishing hand-assembled supplemental receipts from compiler evidence.
- [ ] Repeat original buh startup and record the actual result without successful stubs.
- [x] Verify workspace formatting, Clippy, tests, rustdoc, consumer closure and core dependency trees.
- [ ] Archive the change only after verification.

- [x] Recover measured left substring through existing BSL and reject malformed operands.

- [x] Recover measured backward boolean assignment joins and reject unsupported incoming paths.

- [x] Recover measured empty handlers in functions with validated return tails.

- [x] Recover measured dynamic private function lookup via the existing evaluator and reject malformed operands.

- [x] Recover measured opcode 32 operand 1393 and replay the complete native control without changing existing test expectations.

- [x] Recover measured value parameter flag 13 with slot 0 and verify supplied and omitted calls against the native receipt.

- [x] Recover the complete measured overlapping constant tail while retaining both ordinary and private async declarations; protect other entries and malformed tails.

- [x] Recover measured opcode 66 through the existing integer-part builtin and replay value, reference and snapshot receipts; reject malformed operands and missing input.

- [x] Recover measured separate initializer/module termination and verify three-instance storage and corrupt terminal rejection.
- [x] Preserve and replay the independent supplemental native rounding result-capture control before a reference-mutating call.
- [x] Measure month-start result capture before a reference-mutating call and reproduce the delayed-evaluation mismatch before fixing opcode 76.
- [x] Measure all twelve supported calendar boundary result captures and reproduce the eleven delayed-evaluation mismatches before fixing them.
- [x] Measure add-month result capture and reproduce routing rejection and delayed-evaluation mismatch before fixing opcode 75.
- [x] Measure three-part and six-part date construction result capture, remove locale-dependent number grouping with a second native receipt, and reproduce both delayed-evaluation mismatches before fixing opcodes 73/74.
- [x] Measure Min, Max, Type and Format result capture and reproduce all four delayed-evaluation mismatches before fixing opcodes 103/104/108/110.
- [x] Measure and replay Mid, StrReplace and Eval result capture before reference-mutating calls without changing already-correct recovery.
- [x] Measure and replay numeric conversion result capture for a string argument before a reference-mutating call without changing already-correct recovery.

- [x] Recover both functions in native-image-try-return-compiled.image and replay all four native lines; reject corrupt handler, join and cleanup tails.

- [x] Recover both static private flag-2 controls and replay all six native lines; reject dynamic evaluation, corrupt arity and unsupported body boundaries.

- [x] Replay both native private-procedure-eval-outcome lines through recovered BSL while retaining the full native diagnostic separately.

- [x] Recover compiler-produced opcode 52 and replay both native Execute lines; reject wrong operands, missing input and residual stack state.

- [x] Replay both native private-procedure-execute lines through recovered BSL, preserving the compiler baseline and supplemental provenance separately.

- [x] Recover the measured shared-tail-both control and replay its four native lines; reject corrupt jump, cleanup and conflicting entries.

- [x] Recover shared-reference-body and replay four native lines; reject changed frames, jumps, shared operations and exits.

- [x] Replay private-variable-slot-only and reject an unknown private reference external slot.

- [x] Measure shared-try-body in an isolated native infobase, replay both entries and all four receipt lines, and reject changed guards, frames and interior entries.

- [x] Measure public traversal of an overlapping entry with the observed unknown frame flags, preserve the independent private-call process failure separately, and retain explicit whole-image rejection rather than discard the declaration.
