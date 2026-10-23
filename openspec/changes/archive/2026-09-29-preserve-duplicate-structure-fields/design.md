## Evidence

The 1C 8.3.27.2342 platform probe in
`tests/conformance/fixtures/structure_duplicate_fields.bsl` and its
platform-recorded `.expected` output establish that:
`Structure("Поле,поле", 1, 2)` and its computed-list counterpart both yield
one field with value `2`; `Structure("А,Б,а", 1, 2, 3)` iterates as `аБ` and
exposes values `3` and `2`; replacing `А` with `Insert("а", 2)` iterates as
`а`. The platform XML export of UNF contains 19 literal duplicate-field
constructors.
The same probe confirms that a FixedStructure copy and an internal-string
round trip retain `а`, a second Structure remains `А`, and both duplicate
value expressions execute in source order.

## Implementation

The resolver retains `NewStructure` only when the literal field list has
distinct names under `folded_eq`. A duplicate literal becomes the existing
`NewDynamicStructure` expression with its original arguments, so the VM
evaluates every argument and the verifier never receives a shape with
duplicate `NameId`s. Ordinary literals keep their shaped representation.

Runtime Structure storage keeps an optional per-object spelling for fields
whose displayed key differs from the shared `NameInterner` spelling. This
avoids mutating the process-wide name table when one object replaces `А`
with `а`. Iteration, fixed-structure copy and value serialization read that
spelling; field lookup remains keyed by the same folded `NameId`. A repeated
`Insert` updates the spelling at the existing position. The spelling data is
owned by the Structure and therefore does not cross objects or sessions.

No additional bytecode instruction, crate dependency or database behavior is
introduced. The existing invalid-shape verifier check remains intact.
