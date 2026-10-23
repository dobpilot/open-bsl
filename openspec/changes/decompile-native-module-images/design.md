# Design

The selected database loader already decompresses module resources and reads
their native containers. Extend that reader to select a named entry rather
than add a second container parser. Reuse the existing serialized-value parser
for `image`. Source text remains preferred when available; an image supplies
the single effective BSL body only when source text is absent.

Keep native-image decoding and BSL reconstruction outside bsl-rt and bsl-vm.
Recovered source enters the existing ConfigurationCodeSnapshot path, so
imports, compilation context, extension composition and execution use the
current open-bsl mechanisms. This change does not introduce a second BSL VM
or alter global-name resolution.

The initial evidence is the image-only buh module
aab59d30-8535-4c41-9461-f127db41da91. v8unpack reads its container and
serialized image; the author's v8asm table names every opcode it uses.
Those descriptions are research references, not native execution oracles.
Controlled modules compiled by 1C must establish flags, parameters, stack
effects, references, jumps, loops and exception boundaries before those rules
are implemented. Preserve argument evaluation order, omitted arguments,
reference parameters, short-circuit evaluation and exception behavior.

Reconstruct structured BSL only when the image structure is supported and
verified. Unknown opcodes, invalid references and unsupported control-flow
forms produce a diagnostic identifying the module and failing location.
Do not emit partial bodies that appear complete. Arbitrary hand-written or
obfuscated native images are not promised to have a structured BSL equivalent.

Keep proprietary configuration images in local research artifacts. Check in
small independently authored native fixtures and their measured receipts.
Retain attribution if external implementation code is reused. Add no external
dependencies; keep public comments and documentation in Russian.

The decompiled source is produced once during snapshot loading and reused by
the factory. It is not regenerated or fingerprinted for each database Session.

During artifact research, the raw-table reader is compiled only for tests in
bsl-config-db. It uses the existing serialized parser and preserves numerical
flags and operands without deriving execution semantics. This staging keeps
the current loader unchanged until native compiled fixtures and BSL
reconstruction are verified. Passing raw-table tests does not count as
implementing image-only loading or native behavioral preservation.

The first loader integration accepts only the whole image forms verified by
the recorded native execution receipts, including the isolated compiled buh
date function. Unsupported whole modules still fail explicitly; this stage
does not claim support for the full buh payroll image. The independently
authored compiler fixture has subsequently been acquired as described below. Existing text remains preferred.
The native container reader is parameterized by entry name and reused for
`text` and `image`; reconstruction failures include the resource identity.

The native compiler control image is now checked in with its source and native
receipt. It was produced by native distribution of the authored common module
without source text. Both sequential CF container segments were inspected; the
64-bit segment contains the matching configuration and module. Its module
resource contains only image and info. Native execution matches all ten source
receipt lines. The compiled image omits its empty global Var section; the raw
reader accepts that omission while retaining strict checks for present tables.
This acquisition does not imply that the full control image is reconstructed
yet; it supplies the independent compiler evidence for that implementation.

Linear reconstruction now reads parameter passing flags, defaults and indexed
locals from the compiler declarations rather than matching an entire arithmetic
body template. The isolated first compiler method has a native execution
receipt, including its omitted default argument and reference update. Method
boundaries, terminal instructions and operands are validated before any complete
module is returned. Unknown control-flow forms remain explicit failures.
The user-provided v8asm article and source are research aids for subsequent
control-flow measurements, not substitutes for native execution receipts.

Structured return branches from the first three compiler methods are also
verified against the complete native receipt. Alternative targets, join targets,
return cleanup and the common method exit must match the measured structure;
malformed targets fail explicitly. This does not accept arbitrary control-flow
graphs or claim reconstruction of the remaining compiler methods.

The first four compiler methods now also cover the measured counted loop,
while loop and collection iteration. Reconstruction validates temporary-stack
setup, exit and back-edge targets and iterator cleanup. Compiler-generated
iterator names are mapped to valid private source names; removing iterator
bookkeeping requires that the internal slot is not otherwise observed in the
body. The reconstructed loop result is compared with the complete native
receipt. Arbitrary nesting, break and continue remain unsupported.

All seven methods of the independent compiler control image now reconstruct
and execute through the existing facade, compiler and VM against all ten native
receipt lines. Short-circuit reconstruction retains the right-hand local call
inside the generated BSL expression so skipped operands retain no side effects.
The measured try/raise/caught-return form validates handler and cleanup targets;
corruption of the final handler rejects the entire module. A separate resource
loader test uses the complete native Module.bin and verifies private visibility.
This evidence covers the control fixture, not the complete buh image.

The real buh method table is not ordered by body address. Body boundaries now
use the next greater ordinary method entry while preserving the original table
and call indices. Reordering independent declarations in the control fixture
is checked against all native receipt lines; duplicate ordinary body entries
are rejected. Unsupported external method declarations remain explicit errors
and do not define ordinary body boundaries. Measured external global references
are accepted by the structured path and resolved by the existing BSL resolver;
they are not redeclared as module variables.

Branches with a shared continuation are reconstructed separately from the
all-returning branch form. Completed condition-preparation statements retain
their order before the generated conditional. Both paths are checked against
the existing native reference-update fixture. Logical negation is supported
only after a separate native image receipt verifies opposite path selection
and parameter updates. Unsupported local call arities remain explicit errors.

A separate native fixture verifies a local call omitting an optional argument
whose declaration default is U, explicit argument transmission, and a nested
local call to an exported zero-parameter function. Reconstruction emits the
default declaration and actual call arity for the ordinary compiler and VM;
it does not replace the call with a successful result. Omitting a mandatory
parameter still fails explicitly. Other unverified call forms remain errors.

Separate native receipts now verify indexed array reads, indexed assignment
with caller-visible mutation and catchable out-of-bounds failure, as well as
the three-way numeric boundary for <=, <> and >=. Reconstruction uses ordinary
BSL indexing and comparison operators, preserving existing runtime errors.
Reserved operands are rejected. This advances decoding of the first buh method
but does not establish execution of the full module or configuration session.

Native context-call receipts cover two arguments with a reference output and
a four-argument column-add call. Two-argument execution matches the native
receipt. The four-argument call reconstructs with the correct order but cannot
yet match native execution because the ordinary runtime also rejects that
column-add arity. A separate follow-up records this runtime limitation; tests
compare reconstructed and ordinary-source failures without claiming successful
native equivalence. The complete nine-line native receipt remains preserved.

A separate native fixture verifies procedure and function entries marked 32
and 33 with no body, local table or default table. Their original indices remain
available to CallLoc; reconstruction emits ordinary global calls and no method
stubs. Existing runtime/host resolution supplies implementations. Native target
mutation and localized return match the reconstructed execution. Unsupported
external declaration shapes fail before a complete module can be returned.

Mixed forward branches and while back edges are distinguished by validated
targets instead of classifying every conditional in a loop-containing method
as a loop. A native fixture covers iteration, zero iterations and skipping the
loop branch with reference updates. A separate false-condition receipt confirms
that a previously rejected backward jump is a valid loop form; its potentially
unbounded true path is not executed or claimed as measured.

Native receipts verify date construction from a string, three calendar parts
and six calendar/time parts, including leap day and the result type. The image
instructions reconstruct to the existing Date function with argument order
preserved. Nonzero reserved operands fail explicitly; no separate calendar
implementation or native-bytecode execution path is introduced.

Counted-loop bodies use structured branch reconstruction after a native fixture
verifies both inner alternatives, caller-visible sum and iteration count, an
empty loop range and skipping the enclosing branch. Unsupported branch joins
and back-edge targets still fail explicitly. This does not claim support for all
control-flow combinations or successful original configuration startup.

A native zero-argument context-procedure fixture verifies clearing an array,
the returned count and the mutation visible to its caller. The decoder accepts
this measured arity and emits an ordinary object-method call; it introduces
no separate object dispatch or native execution mechanism.

Structured reconstruction also applies to procedure bodies, whose existing
end validation still rejects function returns. A hand-assembled native fixture
verifies an exported procedure with an enclosing branch, a counted loop and
an inner branch, observing both reference parameters for iteration, an empty
range and a skipped branch. It reuses the same reconstruction helpers as
functions rather than adding a procedure-specific control-flow engine.

Collection-loop bodies reuse structured branch reconstruction after a native
fixture verifies both branch alternatives, preservation of the input array
and an empty collection. Iterator scaffolding and cleanup validation remain
unchanged; invalid inner branch, continuation and loop-exit targets reject
the complete image.

Native fixtures verify three-argument string replacement, including overlap,
Unicode, absent matches, deletion and an empty search string, preserving the
source reference parameter. Reconstruction invokes the existing StrReplace.
Another native fixture verifies a one-argument context function with successful
and absent array lookups; existing call-result snapshots also cover this arity.

When a method call obtains a foreach collection, its result snapshot belongs
to the loop prelude. A native fixture verifies column extraction from a value
table before iteration, both inner branches and an empty table, while preserving
the original table. The existing single-expression collection check remains.

Native receipts verify instructions for beginning and end of month, quarter
and year on leap day, year end and quarter start. They reconstruct to existing
calendar functions. All six reserved operands must remain zero; no separate
calendar implementation is introduced.

Native context-procedure receipts verify three, five and six arguments,
preserving the order and caller-visible reference updates. Local calls use
the existing ordered argument extraction for arbitrary declared arity and
accept omitted trailing parameters only when each has a default. Native
receipts verify multiple arguments, omitted defaults and a zero-parameter
procedure with a caught exception; invalid counts and required omissions fail.

Context-function receipts verify every arity through six, optional reference
parameters and aliasing two parameters to one caller variable. Constructor
receipts verify every arity through six using multidimensional arrays, including
mutation and independence. Constructor evaluation still occurs at the original
instruction through a result snapshot and the existing BSL constructor.

A native while fixture mutates its input array, uses both inner branches and
observes reference updates, verifying that the context call in its condition
is repeated before every iteration. Such calls remain inside the reconstructed
condition instead of a one-time prelude. Ordinary calls retain their existing
snapshots; invalid loop and branch boundaries still reject the image.

The native Format instruction reconstructs to the existing Format function.
Receipts verify dates, ungrouped numbers, booleans, strings and Undefined.
A separately recorded existing formatter difference for a single grouping size
is reproduced by both ordinary and recovered BSL. The full native receipt is
retained and is not claimed as a complete match; no formatter behavior is fixed
as part of reconstruction.


The supplemental native receipts now cover early procedure exits through
`45,0` and a jump to the same method's validated terminal line, type operations
107/108, and day/hour/minute boundaries 93–98. These map to existing BSL
statements and runtime functions; no additional execution engine is introduced.
Weekly operations 91/92 remain rejected: the file-infobase receipt and direct
native BSL calls agree with each other but differ from the existing server
calendar oracle. No calendar-context rule is inferred from those receipts.
The complete image is rejected rather than returning only its supported methods.


The supplemental min/max receipt measures instructions 103/104 with counts
1 through 6 supplied by instruction 18. Recovery retains the original argument
order and delegates comparison to the existing runtime functions. Missing,
invalid or unsupported counts and reserved operands reject the entire image.


A supplemental native receipt distinguishes instruction 10 in a context-function
argument position from explicit Undefined. Recovery preserves an omitted BSL
argument so the existing compiler/runtime supplies the default and reference
updates do not mutate the caller's omitted slot. The marker is rejected as an
ordinary expression value, and other unmeasured call forms remain unsupported.


Further native receipts verify division/remainder (27/28), boolean defaults
`B,0` and `B,1` only in DefPrm, eight-argument local calls with omitted
positions, and seven-to-nine-argument context calls. Constructors retain their
existing supported range. Discarded function results remain harmless in an
ordinary statement block because each call has already been emitted as a
statement; the pure-expression path still rejects discarding an unexecuted
inline call. Reading the native result register after a procedure remains
unsupported rather than being assigned an inferred value.

A combined supplemental receipt covers Mid (59), Round (67), Month (83)
and Day (84). These map to existing runtime functions. An integration test
filters a synthetic Config resource set, loads the independently compiled
control Module.bin through the configuration snapshot, constructs the facade
graph from that snapshot, and verifies its original ten-line native receipt.
It does not substitute for the still-pending original buh startup or expanded
independently compiled control fixture.


Collection-loop continuation is recovered only when its jump targets the same
loop's already validated final backward jump. Branch recovery carries that
continuation target into the loop body and replaces it at a nested collection
loop. Other loop kinds do not inherit the target inside their bodies, while
statements before or after a nested loop retain their enclosing context.
Recovery emits the existing BSL Continue statement; the existing compiler and
VM perform iteration and cleanup. Arbitrary jumps and unmeasured break/continue
forms remain unsupported, and unsupported instructions after a continuation
cannot be hidden as unreachable code.

### Conditional expression with a pending assignment

The native `conditional-expression` probe preserves a reference assignment
target below the condition and yields one value at the forward join. Both
branches match independently authored BSL; the unselected division by zero is
not evaluated. Recover this form as the existing BSL conditional expression.
Require each branch to yield one pure expression and validate both forward
boundaries. Do not move branch statements outside the conditional.

### Nested conditional return

The additional native `conditional-return` probe has an outer and an inner
conditional sharing the return join. There is no pending assignment target.
Recognize expression alternatives by verifying both as single expressions
and then reconstruct the complete surrounding block. Statement alternatives
retain the existing structured branch path. Reject malformed boundaries and
unsupported commands instead of dropping a branch.

### Collection-loop break

The native `foreach-break` receipt compares the iterator-cleanup jump with
independent BSL `Break` for mixed, empty, immediately stopped and uninterrupted
collections. Carry both the validated iteration-backedge and cleanup targets
through structured branches. A nested collection replaces these targets;
unsupported while/counted-loop exits retain explicit rejection. Do not discard
unknown instructions after an exit.

### While-loop break and adjacent forward jump

The native `while-break` receipt covers immediate break, break after several
iterations, normal termination and skipped entry, comparing all parameter
updates with independent BSL. A validated while loop carries its exit target
through the body; its unmeasured continuation target is absent. An adjacent
forward jump at an empty statement boundary advances by one instruction and
does not skip code. No arbitrary forward jump is admitted by that rule.

### Multiple function returns

The native `function-early-return` probe has two conditional early returns
and one final return with the same method exit. Relax the single-return
restriction only for a structured function. Validate block-close and forward
exit instructions against that function, and reject any unsupported tail
in the reconstructed expression range. Linear unreachable code remains
rejected. The normal BSL compiler handles return execution.

### While-loop continuation

The additional native `while-continue` receipt now verifies the previously
unmeasured while continuation. It jumps to the validated beginning of the
condition, skips accumulation for one iteration and preserves the decrement
and visited count. Both supported loop kinds can therefore carry a concrete
continuation and exit pair. This does not enable counted-loop continuation
or arbitrary targets inside the condition.

### Counted-loop limit call

The native `counted-limit-call` probe obtains an array count before iteration
and appends elements in the body. Empty, single and three-element arrays
match independently authored BSL, including the final counter. Recover
the call's existing expression prelude before the counted loop. Keep
its returned temporary in the bound expression; do not inline the call
into the repeated comparison or move the prelude into the body.

### Statement try and error information

The native `statement-try` receipt compares normal execution and an explicit
throw inside a procedure, including prefix, handler and suffix assignments.
The supported block has a validated handler entry, normal block close and
forward jump, and a handler-end instruction immediately before continuation.
Recover body and handler recursively as existing BSL statements. Preserve
the measured error-information instruction as an eager snapshot through
the existing `ErrorInfo` builtin. Unsupported boundaries and commands remain
errors; no new VM exception machinery is introduced.

### External procedure omissions

The native `external-omitted` receipt verifies external procedure flag 32
using `FillPropertyValues` with two omitted optional positions. Preserve
the existing omission sentinel when reconstructing external procedures,
letting the normal resolver and runtime own optional-parameter behavior.
External-function omissions remain rejected: the broader initial probe
produced no receipt and is not compatibility evidence.

### Final module terminator

The independent compiler fixture already measures consecutive method and
module end instructions. The real buh image has the same suffix after its
last ordinary function. Apply that verified boundary in the common method
end calculation rather than only in the special caught-return path.
Only the exact pair of zero-operand end instructions is recognized.

## Preserve computed values before subsequent calls

The native `measure-native-image-expression-snapshot` receipt proves that
`(Argument + 2)` is computed before a subsequent call modifies `Argument`.
Reconstruction must preserve this value at its instruction, rather than emit
an expression that reads the operand after the call. Reference expressions
remain references. Expressions reconstructed inside short-circuit paths must
still execute only on the selected path.

The supplemental `measure-native-image-builtin-snapshots` image measures
commands 59, 67, 72, 82–84, 100 and 107 before a mutating reference call.
All eight native results equal direct BSL; deferred reconstruction previously
failed all eight comparisons. These measured commands use the same eager
result capture as scalar operators. Locale-independent formatting is confined
to the probe output; it does not change application execution.

The next read-only buh acquisition reaches command 61 in another image-only
module. `measure-native-image-upper-snapshot` proves that this command invokes
uppercase conversion and captures its value before later operand mutation.
Recovery emits the existing `ВРег` builtin; no string conversion implementation
or VM instruction is added. Reserved operands and stack arity remain checked.

The next buh image reaches command 58 and also contains command 65.
`measure-native-image-remaining-string-snapshots` measures both before operand
mutation and compares with direct BSL. Recovery emits existing `Прав` and
`ПустаяСтрока` builtins, capturing their results at the command. Reserved
operands and argument stack bounds remain checked. No new runtime API is added.

Command 71 is reached next in the same original image. The native
`measure-native-image-string-snapshot` converts 12 to text before mutating
the reference to 10; its result agrees with direct BSL. Recovery uses the
existing `Строка` builtin and captures its result. This does not add or redefine
formatting rules, and the numeric probe avoids locale-specific display.

External function omissions are now measured in two supplemental images:
`ПоместитьВоВременноеХранилище(, UUID)` reproduces the next original buh
call and `НСтр(Text,)` supplies executable facade coverage without a host
service. Both agree with direct native BSL. Flags 33 therefore preserve empty
argument positions just like local calls and external procedures. Declared
maximum counts remain checked; unknown procedure flags remain rejected.
The older prototype test rejecting flag 33 omissions is replaced by rejection
of an unknown flag; it was authored for this change, not a baseline regression.
The existing runtime accepts only string addresses in the second temporary
storage argument, so UUID ownership remains a separate reached runtime gap;
this reconstruction change does not fabricate support for that argument.

The next original image contains one ordinary body plus one external function
record. The dispatcher wrongly selected the old two-local-method template.
This is an internal bug fix under the already specified external-call recovery:
external declarations select the general reconstruction regardless of table
size. A derived NStr regression and the real image's full syntax check pass.
No additional native opcode or platform semantic rule is introduced.

The fifth original image reaches command 56 and also contains command 53.
`measure-native-image-length-trim-snapshots` measures both before reference
mutation. Its non-BMP string has native length 4; trimming returns the saved
text. Recovery invokes existing `СтрДлина` and `СокрЛП`, captures their
results and retains operand and stack validation. No Unicode algorithm is
introduced or replaced.

The next original method mixes conditional assignments with later statement
branches. Returning the whole range to expression recovery after recognizing
a pure conditional misclassifies subsequent statements. Statement scanning
therefore skips a validated pure expression's join and continues searching for
statement control. Validation reuses the existing pure-branch recovery; all
prefix expressions still execute in their original order. The supplemental
`measure-native-image-mixed-conditional` provides native/direct BSL evidence
for positive, zero and negative reference arguments.

`measure-native-image-counted-loop-exits` verifies continuation and break
for starts -1, 0, 1, 3 and 5, including reference-counter values. All ten
native cases match direct BSL. Counted-loop recovery passes its already
validated increment and cleanup targets to the existing statement recovery,
just as collection and while loops do. Nested loops replace the enclosing
exit context, and unsupported jumps remain explicit errors.

A sixth read-only buh image has the SCSU signature `0e fe ff`. ICU converts
its complete serialized tables to UTF-8 without replacement characters.
The native `measure-native-image-scsu` receipt checks all ten results of the
independently compiled seven-method control image after ICU re-encoding to
SCSU. This measures native acceptance of the encoding; it does not claim
that 1C itself generated this compressed variant.

Recognize the measured SCSU signature only. Decode the complete standard
according to Unicode UTS #6 before reusing the existing serialized parser.
The private decoder remains in bsl-config-db, adds no dependency, and does
not change the runtime's public BSL encoding capabilities. Reject reserved
tags, incomplete arguments and invalid surrogate sequences instead of
substituting characters. Test mode transitions, static/dynamic and extended
windows against independent ICU vectors. Unsupported decoded control flow
continues to fail normally; successful transcoding is not module support.

Native probes `measure-native-image-table-count-*` correct an initial parser
assumption: numeric table headers are not mandatory record counts. Eleven
independent higher/lower variants of Cmd, Const, Proc, global/local Var and
DefPrm match all 330 control receipt lines on 8.3.27.2342. Their purpose
beyond this observed lack of cardinality enforcement remains unspecified;
do not call them array capacity or invent padding records.

Validate the unsigned header token and read the actual complete record list.
Existing shape checks, parameter declarations, indices and body validation
remain in force. Update the rejection assertion in the raw-reader prototype
created during this change to check malformed tokens and structures instead
of the now-measured valid mismatch. No baseline regression test is changed.

The native `measure-native-image-label-tables` probe supplements the counted
loop image with root and ordinary-method Lbl sections and SCSU encoding.
All 30 results match the native control. Accept only the measured record
shape: a string and an unsigned numeric field. Validate every actual record
and reject duplicate sections. The numeric field's purpose is not inferred.
The existing supported bodies reconstruct control flow from validated jump
instructions; unused label metadata adds neither branches nor source stubs.
This does not introduce recovery of arbitrary goto graphs or permission to
skip unknown instructions.

The source and hand-assembled `measure-native-image-module-initializer`
controls return the same seven native lines: a private variable is set to
42 on object construction, independently changed to 111 and 222 in two
objects, and a third object starts at 42. The image uses global variable
flag 0, load-global opcode 2 and initializer entry 13 after ordinary bodies.
This is native execution evidence for the supplied image, not an independently
compiled object image.

Recover the supported initializer as ordinary top-level BSL, with private
module declarations, through the existing compiler and per-object memory.
Include the initializer entry in ordinary method boundary calculation so
it cannot silently become part of the last procedure. Validate the complete
initializer and all method bodies before returning any source. Do not add
another execution engine or change facade instance ownership. Independent
compiler evidence for the extended control fixture is still required.

The same source and native image also execute as a Catalog object module;
`measure-native-image-catalog-module-initializer[-source]` records seven
matching lines. The facade comparison uses the existing bsl-session object
module integration with three objects in one Session, repeated in a second
Session, rather than substituting independent root States for object frames.
The session fixture declares the catalog constructor dependency in its root,
so the existing configuration closure includes the object module.

This measured phase supports a trailing initializer and flag-0 private module
variables in that form. Unmeasured flag-0 declarations without an initializer
retain their prior rejection. Explicit native global loads whose name would
be shadowed by a local in emitted BSL fail rather than silently changing scope.
The ordinary resolver's variable priority and facade instance caching remain
unchanged. Corrupt final initialization rejects the entire recovered module.

The hand-assembled `measure-native-image-no-lines` control executes on native
8.3.27.2342 without opcode 1. Its five receipt lines cover a constant function,
three statement-branch paths and a reference-writing procedure. Support the
measured function tail Return/Exit/Jump/End with an optional existing Line
before End. Compute the exit boundary from the actual tail, retaining exact
Exit operands and Jump destinations. Do not extend unmeasured caught-return
or all-branches-return tail forms as a side effect. This probe is execution
evidence for the supplied image, not independent compiler output.

The hand-assembled `measure-native-image-jump-true` probe supplies ten native
receipt lines for opcode 41: statement branches, conditional expressions,
reference updates and an unselected division-by-zero branch. Reconstruct the
measured forms using a negated BSL predicate while retaining the existing
branch boundaries and operand evaluation order. This does not permit arbitrary
jump graphs, skipping unknown instructions or unsupported loop/return forms.
The native receipt proves execution of this research image, not an independent
compiler image. Existing ordinary name resolution is unchanged.

## Measured unreachable regions

The hand-assembled `measure-native-image-unreachable-probe` receipt supplies
four native results. A constant false/false equality followed by JumpTrue
skips invalid operands; a constant false JumpTrue does not enter a different
method; a forward Jump skips invalid operands. These are native execution
controls rather than assumptions about unused configuration methods.

This evidence narrows the earlier blanket refusal to omit unknown commands:
reachable unknown commands remain failures, while a proven unreachable
region may be omitted without returning a partial executable module. All
method declarations, table records, initializer and entry boundaries still
require validation. Never infer nonexecution from a method name or from not
having called that method during a test.

Implement only local, pure constant-predicate and forward-jump normalization,
retaining original instruction addresses for diagnostics and existing branch
validation. Protect every entry and incoming control-flow edge; a possible
edge into a supposedly skipped region prevents its removal. Unknown or
mutable conditions stay untouched and must pass ordinary complete-body
reconstruction. This is a compiler preprocessing pass, not another VM.
Tests must exercise native receipts and reachable corruption, incoming edges,
nonconstant predicates and malformed destinations before broadening it.

The first measured normalizer accepts only contiguous False/False/Equal/JumpTrue,
False/JumpTrue and forward Jump forms. It retains every original instruction
address by replacing proven skipped commands with existing empty Line commands
inside the recovery image, after raw table parsing. Raw parser results are not
changed. Return/handler exit tails remain intact. Original jump edges and all
procedure entries protect interiors of both predicates and skipped regions.
Sorted destinations bound lookup to the candidate region; other conditions
are left to ordinary reconstruction. The native Module.bin control executes
all four exported methods through the existing resource path and facade.

## Method-local label jumps

Native `goto-labels-local` and `goto-labels-both` hand-assembled controls give
42 and 22 for two methods starting at 0 and 8, whose local label fields are
3 and 11. A distinct unused global table does not change those receipts.
Opcode 42 uses a method-local label index in these measured forms; the
numeric target is an absolute command address. Opcode 43 is not accepted as
an absolute jump merely because it neighbors opcode 42 in the opcode table.

Retain local label targets when parsing, resolve measured forward label jumps
within the ordinary method boundary and feed resolved jumps to existing BSL
reconstruction. Do not infer initializer/global label behavior. The root-only
negative probe crashed native 1C without a receipt; it is not a supported
shape or a compatibility promise. Validate references and keep malformed
reachable targets as diagnostics. No new VM or resolver priority is needed.

Native `boolean-skips` supplements the constant-predicate controls with True,
negation of True/False and both operand orders of boolean equality. Six receipt
lines also exercise impossible branches into another method's predicate.
Extend only these pure literal prefixes. Revisit guarded candidates after
other proven false jumps have been removed; an eliminated edge is no longer
an executable incoming edge. Keep live edges, parameters and opaque conditions
as barriers. This is local normalization, not execution of arbitrary bytecode.

The native `goto-backward` control returns 3 after three iterations using a
method-local label address 3 from command 12. Resolve this measured back edge
through the same local table and let the existing structured-while recovery
handle it. Targets must remain inside the ordinary method; arbitrary goto
graph reconstruction, self-jumps and initializer label resolution are not
introduced by this control.

Native `unreachable-cycle` preserves all four earlier receipt values while
two skipped regions jump into each other. Raw incoming edges alone therefore
cannot decide reachability. Add a conservative worklist from every declared
entry and the initializer, carrying only the measured boolean literal suffix,
negation and equality facts. Join conflicting facts to unknown; fork both
paths for unknown predicates and handler entries. A reachable unresolved label jump remains an explicit unsupported barrier;
it is retained and must fail complete reconstruction before any source is exposed. Use this proof only to disregard impossible
incoming edges and omit regions; ordinary reconstruction remains responsible
for the entire source and rejects reachable unsupported commands. This pass
does not execute calls, inspect host state or implement another interpreter.

The native `next-jumps` control gives four receipt lines with a next-command
jump after a value, in a condition and above a reference assignment target.
Permit this measured no-op in those expression states without changing the
stack or argument evaluation order. A jump
to another command must not be generalized into a no-op. Return and handler
tails continue to be checked by their existing structural validators.


Native opcode 32 is labelled `Unused` in the historical assembler table,
but the current platform executes a boolean conversion. The hand-assembled
`opcode32-values` control records 22 lines: booleans, zero and nonzero
numbers, Russian/English boolean words with whitespace, rejected numeric
words, ordinary strings, undefined and null, and a reference assignment.
Reconstruct this instruction through the existing `Булево` builtin; add no
VM opcode or conversion implementation. Preserve the ordinary expression
capture discipline so conversion happens before subsequent mutations.
Nonzero instruction operands and missing input values remain diagnostics.
This evidence is supplemental, not an independently compiled fixture.


Native `method-flag2` measures private flags 2/3 and exported flags 18/19.
Its promise type assertions and reference updates show that bit 2 must be
retained as `Асинх`, alongside the existing function/export bits. Recover
these declarations through the existing async parser, compiler and VM;
introduce no independent promise or scheduler. Preserve the raw flags in
parsed artifacts. Validate every body and local label using the same
complete-module boundaries; unknown combinations remain unsupported.
This does not authorize ignoring malformed asynchronous declarations or
accepting unmeasured suspension instructions.


The source-level `async-source-parameters` control rechecks the existing
`ASYNC.PARAM.BY_VALUE` anchor: the caller retains 42 and awaiting the
function yields 99. Open-bsl matches both native lines. This differs from
the manually assembled server image with asynchronous flags and reference
scalar parameters. Such a synthetic image is not evidence that the
existing BSL parameter semantics should change; a plain `Асинх` lowering
cannot preserve that hand-assembled reference effect. Require a matching
compiler control for supported async bodies, and reject nonrepresentable
forms explicitly. The ordinary module-call restriction and background-job
policy are not removed by these supplemental probes.


Native `short-circuit32` records eight lines for conjunction/disjunction
with a right operand that increments a reference counter and returns true.
Accept the measured opcode-32 conversion at the right-operand boundary
alongside the existing compiler-control form 69, with a zero operand in
either case. Reconstruct the expression with the existing BSL `И`/`Или`
operators so the right call remains conditional; do not execute it during
decoding or move it into an unconditional prelude.


Native `constant-short-circuit` records four results (42/11/42/11)
for pure literal conjunction/disjunction and includes invalid instructions
in a region skipped on every path. Extend the conservative boolean fact
worklist for measured short-circuit edges and identity conversion of
already-boolean values. Only the measured pure literal prefix may be
omitted before a conditional branch; preserve side-effectful expressions
and actual incoming edges. Implementation is pending while the preceding
workspace gate runs.


### Collection-loop line marker and back-edge trampoline

The hand-assembled `foreach-no-line-trampoline` image removes the source-line
instruction immediately before the iterator check and adds one jump between
the previous back edge and that check. The existing continuation driver,
executed with this binary override on the disposable native file database,
produced the same 24 lines, including all four independent BSL comparisons.
The saved driver and receipt retain the original measurement IDs.
A second variant, with the last jump targeting the preceding back edge as
in the saved buh sequence, crashed the native process without a receipt.
That variant is not treated as equivalent or supported.
Recognize only the measured direct or single-trampoline boundary and the
optional source-line marker. Keep iterator identity, assignment sequence,
cleanup and nearest-loop continuation validation intact; no arbitrary
jump-chain traversal is introduced.


The failed backward-tail control had a generator address error: the old
line-marker target was mapped to the preceding assignment. After mapping
that removed marker to the next iterator-check instruction, the corrected
`foreach-backward-tail` produced all 24 expected native lines. Its terminal
jump targets the preceding validated back edge; both jumps are excluded
from the reconstructed body and continuation targets the first back edge.
The earlier crash therefore is not evidence against this corrected form.


### Renamed private iterator storage

The native `foreach-renamed-iterator` control changes only the private iterator
name and reproduces all 24 lines. Validate its local-storage flag and the
same slot across initialization, selection, advance and cleanup, including
the existing prohibition on body access to hidden storage. Its spelling is
not a semantic constraint. Keep the element variable's existing name for
reconstructed BSL.


### Native numeric conversion

The hand-assembled `number-conversion` receipt measures opcode 70 with
numbers, numeric strings, Boolean values, invalid values and a reference
assignment. Lower it to the existing `Число` builtin. Capture its result at
the command boundary through the same temporary mechanism as other measured
conversions; no numeric semantics are implemented in the decoder.


### Compact conditional loop break

The native `foreach-compact-break` control removes only the redundant
forward join following a conditional break and reproduces all 24 receipt
lines. When the last command of the selected branch targets the validated
enclosing-loop exit, retain it in the branch body for existing Break
recovery instead of interpreting it as a generic forward join. Ordinary
joins retain their existing bounds. Unknown exit targets remain rejected.


### Procedure exit directly to the terminal command

The native `procedure-exit-no-line` control removes the last line marker
from the measured early-return image, keeping the jump address at the
terminal End command. Both reference receipts remain 10 and 20. Accept
only the current method's final End or its immediately preceding positive
line marker. Keep the existing empty-stack, method-kind and tail checks.


### Compact conditional procedure return

The hand-assembled `procedure-compact-return` removes only the redundant
forward join after the known early-return exit jump. Native reference
results remain 10 and 20. The structured branch can exclude the validated
exit jump as its join boundary; in that case the Return command is the
last command in the selected body. Check the exit against the whole image
and treat only this exact branch-boundary tail as empty, retaining rejection
of unknown commands in any actual tail.


### Dead structural join after early procedure return

The native `procedure-dead-join` changes the redundant forward join after
a validated early return to target the middle of the alternative assignment.
Both native reference results remain 10 and 20. Such a join does not delimit
the alternative's prefix: it cannot execute after the prior exit. Reuse
conservative reachability with all declared entries to prove the last jump
unreachable, and retain normal recovery of the return and both branches.
Do not ignore reachable joins, other entries or unknown branch commands.


### Single-jump target normalization

Two native collection controls prepend a skipped trampoline and route either
the false-condition branch or the loop back edge through it. Both reproduce
all 24 independent BSL receipt lines. Retarget only the measured opcode-40
branch and backward opcode-39 edge through one unconditional same-method
jump to a non-jump destination. Collect edits before applying them so the
pass cannot accidentally follow chains. Preserve the trampoline instruction
itself and compute reachability after these equivalent target edits.


The target pass is limited to backward targets demonstrated by the two
controls; the immediately preceding back-edge pair remains structural.
Forward branch joins retain their existing representation. Regression
controls exposed that changing those unmeasured shapes would disturb
continuation recovery, so those shapes are explicitly excluded.


### Terminal iteration branches

The native `foreach-terminal-branches` control routes the false branch
straight to the iterator check and the true branch to cleanup. All 24 lines
match independently authored BSL. Normalize only proven skipped commands
between a break jump and the final validated back edge, preserving both
structural jumps. A false-condition target equal to that back edge's
validated check reconstructs as Continue in the alternative branch. Other
backward conditional targets keep explicit rejection.


### Known dead prefix after a validated procedure exit

The native `procedure-dead-condition` inserts a conditional and assignment
between a validated early exit and the existing branch join. Reference
results remain 10 and 20. Normalize only the proven unreachable prefix up
to the next executable command after a reachable, validated procedure exit.
Retain the current rejection of unknown return tails; this is not a generic
pass deleting arbitrary unreachable instructions. Other declared entries
and incoming paths participate in the existing conservative reachability.


### Ordinary BSL labels for shared procedure blocks

The `procedure-shared-loop` native image places the looping body between an
early-return path and initialization, then branches back into that shared
body. Reference receipts are 10 and 3. Independently authored ordinary BSL
with labels and Goto produced identical bytes on native 1C and open-bsl.

Keep the existing structured reconstruction first. For supported procedures
whose shared blocks cannot be structured by contiguous ranges, recover
basic blocks into ordinary BSL labels and conditional Goto. Reuse the same
expression and assignment recovery, including result capture and reference
handling. This is source reconstruction for the existing compiler and VM,
not an interpreter for native instructions or a program-counter runtime.

Limit the first measured path to procedures with ordinary control flow.
Reject temporary iterator-stack and exception-handler operations in this
path. Validate every target inside the current method, require each block
to close its operand state except the consumed branch predicate, and keep
unknown instructions explicit. Do not remove shared executable blocks or
return a partial module. Functions and complex structured constructs retain
the existing reconstruction until separately measured.


### Sealed empty handler block

The native `procedure-empty-guard` prepends the exact four-command empty
try region (handler target, try exit, join jump and handler end) to the
measured shared loop. Reference results remain 10 and 3. Recognize this
region as one sealed source block and emit an ordinary empty BSL Try,
retaining exception-context handling in the existing compiler and VM.
Do not erase it or admit incoming edges to its internal commands. Other
handler shapes remain unsupported in the labelled recovery path.

### Left substring
The handassembled opcode-57 image was executed on 1C 8.3.27.2342 in the disposable file infobase. The four recorded requests confirm left substring recovery through existing `Лев`; no separate string implementation is introduced. Nonzero operands and insufficient expression stacks remain unsupported.

### Backward boolean assignment join
The handassembled `backward-bool` control executes on 1C 8.3.27.2342 and updates its reference parameter to true and false. The original buh Cmd[4126] branches to the false literal at 4020; Cmd[4128] joins the shared assignment at 4021. The receiving reference is already on the expression stack. Recovery must fold the selected literals into an ordinary conditional assignment and retain the continuation, rather than treating either edge as a closed expression block. Interior entries, additional predecessors and malformed bounds remain unsupported until measured.

### Empty handlers in functions
The handassembled `function-guard` control preserves four native substring results with an empty handler before the function return. The labelled recovery may therefore cover functions only when their terminal Return/Exit/Goto/End tail is validated. The tail is consumed by the existing return-expression recovery; empty handlers retain ordinary BSL exception semantics. Unknown commands protected by other method entries remain errors.

### Dynamic evaluation instruction
The handassembled `local-eval` control confirms opcode 109,0: a private function selected by dynamic text returns 42. Recover it as existing `Вычислить`, with the existing expression result capture. The separate async receipt does not authorize translating native async reference semantics; unsupported declarations remain explicit errors.

## Private async constant body
The independently executed `native-local-async-eval` image has an exported synchronous wrapper and a private async function with no parameters, locals or effects, returning a constant. This shape is representable by the existing async BSL model and is separate from native async images with scalar references, whose measured effects differ from source BSL. Recognize the private function only after validating that complete narrow body; preserve the async modifier and its method boundary. Other async headers, nonconstant bodies and parameter tables remain explicit unsupported diagnostics. This does not close the broader async recovery task.

## Boolean conversion operand 1393

The independent native control changes only opcode 32 operands from 0 to
1393 and preserves both boolean results. Accept exactly the measured operand
in addition to zero; arbitrary operands remain outside the acquired contract.
This removes the operand refusal at the shared image Cmd[4367], but does not
prove a valid input stack or reconstruct its private overlapping declaration.
Evidence lives in docs/research/fixtures/native-opcode32-operand.

## Value parameter slot zero

The native-parameter-slot-zero control changes only the slot of the
required value parameter from -1 to 0 relative to the complete baseline.
Both supplied and omitted calls preserve baseline behavior. Accept slot 0
only for a parameter with measured flag 13; other local/reference parameter
slots remain unsupported. Missing DefPrm is not enabled: the independent
missing-table control crashes the native platform on first invocation.

## Shared constant tail with a private async entry

The native-overlapping-constant-tail control proves that declaration entry
order alone does not always delimit method bodies. Recognize only an
ordinary parameterless function beginning with a direct jump to the next
private parameterless async constant-return function with its already
measured five-command tail. Extend the ordinary boundary through that
tail, preserving the async declaration; existing all-root reachability
and incoming-edge checks protect the skipped prefix. Unknown overlapping
frames, parameters or tails remain refused. No native execution engine
or removal of private declarations is introduced.

## Integer-part opcode 66

The author-provided v8asm table identifies 66 as Цел; independent native
controls confirm truncation toward zero, conversion failures, numeric result
types, reference assignment and a result snapshot before a mutating call.
Recover it through Цел(Число(value)) using the existing builtins: the native
control confirms numeric conversion of boolean and string inputs before
truncation. Preserve standalone builtin behavior. Use the same result capture
used by other measured scalar operations. No numeric conversion algorithm
is added. The ordinary-body diagnostic reaches Cmd[4600] only after
excluding corrupted private declarations in a temporary probe; production
loading continues to require the complete image.

## Separate initializer and module termination

The native-initializer-double-end control adds exactly one final End,0 to
the existing initializer control. Its seven instance-storage observations
remain unchanged. Exclude both measured terminal instructions from BSL
statement reconstruction when the final pair is End,0 / End,0; keep the
existing single-terminal shape. Do not strip arbitrary commands or internal
termination instructions. Malformed operands remain whole-image refusals.

The independently compiled try-return image supplies nonempty try bodies with conditional raises, terminal returns within body/handler and a suffix return. Recovery validates the platform cleanup and common function exit before omitting cleanup instructions from reconstructed BSL. Existing structured statement recovery handles the remaining bodies. Unknown nested return layouts remain explicit errors.

Supplemental native controls show synchronous reference effects of static opcode-19 calls to private flag-2 procedures. Recovered ordinary private procedures reproduce these measured calls; they are not translated into source-level async declarations, whose reference behavior differs. Images combining these declarations with dynamic Eval remain unsupported until measured. Body boundaries retain every declared entry; intersecting unsupported forms are not discarded.

Native private-procedure-eval confirms a procedure-as-function compilation error; its full diagnostic is retained separately. The outcome control confirms a caught failure and unchanged reference argument. Existing Eval and the ordinary private procedure declaration reproduce that measured path, so the blanket Eval restriction is removed. Execute remains unmeasured and unsupported for these declarations.

The source Execute control and protected distribution confirm opcode 52, operand 0. Reconstruct it as an ordinary BSL Execute statement; no native opcode execution is added to the VM. Its compiled source and native receipt are preserved separately from supplemental flag changes.

Private-procedure-execute changes only the private flag in the independently compiled Execute image. Native opcode 52 retains the same reference effects, so its blanket restriction with flag 2 is removed. This supplemental flag change is labelled separately from the compiler-produced baseline. External method-table declarations named Execute are not covered by opcode-52 evidence.

Shared-tail-both measures a private zero-parameter flag-2 entry at the final line marker of a returning function. Extend that function boundary only for the exact adjacent Return/cleanup/jump and line/End pattern, retaining the private declaration and rejecting conflicting entries or initialization. This does not authorize arbitrary overlapping bodies.

The supplemental shared-reference-body control measures identical one-reference-parameter frames jumping into the same numeric increment and pure parameter-return tail. Recover the shared instructions for each original declaration. For the private procedure omit only the pure parameter read before Return, retaining the increment and ordinary procedure exit. Restrict the recognition to measured frame/prefix/body/tail and reject conflicting declarations or initialization. Do not infer arbitrary native value-returning procedure semantics.

The private-variable-slot-only control independently measures flag 5 with external slot 0 in a private flag-2 procedure. Admit this parameter form alongside the existing measured value-parameter slot-zero form; do not normalize unknown variable flags or external slots of local variables.


### Shared reference body after empty guards

The supplemental `shared-try-body` control executes both original entries on
1C 8.3.27.2342. Two sealed empty try regions precede the existing measured
numeric increment and pure reference-parameter return tail. Both entry paths
change 5 to 12 and return 12. Reuse `shared_reference_body` recognition with
this exact additional prefix, identical parameter frames and defaults, and
no intervening method entry. Reject altered handlers, cleanup, jumps and frames.
This does not establish arbitrary overlap recovery: the original buh entries
have different local flags and missing defaults, and still fail explicitly.

## Legal position (decided 2026-10-23)

Recovering BSL from a vendor module image is decompilation of someone else's
program. Licence agreements of vendor configurations usually forbid it; the
Civil Code of the Russian Federation (art. 1280) allows a lawful user to
decompile for interoperability under its own conditions. open-bsl does not
judge a particular case, so the project's position is:

- Decompilation runs only on an explicit host request
  (`LoadOptions::decompile_protected_modules`, CLI
  `--decompile-protected-modules`). The default loads protected modules as
  export signatures with stub bodies and recovers nothing.
- The user who enables it is responsible for having the right to decompile
  that configuration; the CLI help says so next to the flag.
- Vendor module images and text recovered from them are never committed.
  Tests and probes use images compiled from the project's own control
  modules; probes that need images of vendor modules (the `original-*`
  inputs under `target/diagnostics/`) run only on a machine that already has
  them and keep them outside the repository.

