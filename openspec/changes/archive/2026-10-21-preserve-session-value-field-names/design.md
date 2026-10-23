# Design

## Verified cause and existing mechanisms
Program names and shape-name IDs are local to a compilation. Configuration compilation already shares one prefix across its modules. RuntimeShapes owns mutable names and shape transitions for one execution. ModuleMemory currently preserves ModuleState only; ModuleInstance starts a new ProgramExecution per call. The object-instance bridge explicitly swaps caller RuntimeShapes while entering a catalog object. ValueGraph materialization intentionally copies/relocates a transferable graph for another worker and cannot implement shared mutable values in one State.

The minimal reproduction distinguishes aligned name pools (pass), a shifted producer pool (cross-entry UnknownField), and same-entry reads (pass). Independent journal isolation distinguishes that field failure from the separate explicit unsupported Document metadata FullName method. Do not conflate them or widen FullName support here.

## Ownership decision
Explicit instances of one State need one coherent runtime field-name domain, owned by that State rather than by a process/thread or the immutable Engine. Preserve live object references; importing a compiled root must not rewrite/mutate existing values or copy their graph. Runtime-created names must remain valid on success and ordinary error. Independent State::run/start retain their prior ownership boundary.

Use the existing RuntimeShapes/NameInterner/ShapeTable machinery and the existing cold swap pattern. Import root name/shape references into the State domain before its body executes; keep the shared immutable catalog prefix and do not clone/rewrite the complete catalog for every root. Only the resident representation may be adapted; the host's compiled Module and serialized bytes stay immutable. The concrete binding helper must account for GetProp/SetProp names and structure-construction shape operands using existing representation/validation facilities, not a second compiler, text round trip or a duplicated instruction-effects table. Audit non-instruction shape/name references and constants before coding.

Runtime state must be returned to its owner on ordinary failure, just as common module state already is. Ordinary module variables and transferred returned parameters may hold aliases/cycles; a deep copy or traversal-based relabeling is not an acceptable substitute. Low-level host-created values outside the State name domain are not silently assigned a guessed origin.

## Validation and limits
First add a new external facade regression with shifted pools and preserve the pre-fix failure. Cover both creation orders, shaped and dictionary structures, dynamic inserted fields, mutation observed by the producer, nested aliases/cycles, repeated calls after exceptions, and configuration/common-module entry paths. Reuse existing State isolation and independent-run tests. Validate bytecode-loaded modules and optimizer agreement.

Do not change the VM dispatch loop unless required by evidence. If hot code changes, run the repository machine-code screen and use alternating A/B for any numerical performance claim. New tests must not weaken old assertions. Re-run the actual journal payload case, full workspace gates, consumer and unchanged RT/VM normal dependency trees. Retain the existing three-configuration startup receipts with their original binary scope; obtain current-source acceptance before final main completion. No archive or completion claim while required checks remain.


## Operand and spelling audit before implementation

GetProp and SetProp carry NameId directly; CallObjectMethod/CallObjectProcedure and GetObjectProp/SetObjectProp also address Program.names through u16 operands. The expanded mutation test exposed the latter paths; all must be rebound with checked conversions. NewStructure indexes its own Program.shapes, not RuntimeShapes.shapes, so a cold import can preserve the instruction's shape index while rebinding that shape's field names. Do not impose a new global u16 shape-index limit on runtime transitions. Existing ShapeTable interning and the caller-shape swap pattern remain reusable.

Both the module name table's effective spellings and a literal Structure's original field spelling must be preserved when another State-resident module has already interned a case variant. Rebinding identifiers alone must not change iteration keys or the string supplied to component member lookup. Audit the existing spelling overrides and dynamic compiler prefix checks before implementation. Compiled Module source tables and engine catalog are immutable; resident adaptation must not eagerly duplicate the catalog.

RuntimeShapes must survive ordinary instance calls as well as transfer between roots; ModuleMemory currently stores ModuleState alone. Audit runtime-created dictionary fields across calls, not just the statically named Structure in the minimal regression. Do not treat a prefix with coincidentally matching IDs as proof.


The existing u16 open-member operand capacity remains authoritative. If a translated name does not fit, binding must return an explicit error before the module body executes, never truncate the ID or reinterpret another name. No new serialized format or widened VM opcode is introduced. Dynamic compilation must use the live runtime name prefix, including fields added earlier in the same call; this does not share the independent run/start contexts.


## Implemented cold binding and actual replay

The State owns RuntimeShapes, and the VM accepts that owner only after successful linking. It returns the tables on ordinary success/error. This avoids constructing a second full table just to swap it out. Legacy independent startup retains its ordinary prologue.

The binding pass also remaps ObjectMethod link type_name/method indices, discovered by the actual document replay and retained typed-component regression. All translated u16 operands use checked conversion. NewStructure shape positions remain local while shape field names are translated. Literal spelling overrides are copied into each created Structure when the State's canonical spelling differs. Per-chunk unbundled debug policy survives finalization.

Source audit finds no object transient constants in normal compiler or text-image construction. Existing worker preparation explicitly materializes transient graphs against its own tables and uses independent execution. This change does not infer an origin or traverse/relabel opaque host-created constant graphs; its live value guarantee concerns values of the explicit State passed through the instance API.

Actual journal replay compares all24 retained native Read/Verify outcomes and reads both Right and metadata.Name through separate compiled roots. Document FullName is explicitly unsupported by the existing measured metadata contract and is kept outside this replay, rather than being silently generalized from Catalog.
