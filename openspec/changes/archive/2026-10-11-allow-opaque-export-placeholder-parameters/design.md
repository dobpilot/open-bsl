# Design

Limit the exception to exported headers declaring a positive number of parameters, with no DefPrm table, enough available named variable records whose flags are opaque (neither5 nor13) and whose external slots are0. Continue validating identifier syntax and case-insensitive uniqueness. Emit all declared parameters without defaults or a value-passing prefix; empty bodies cannot modify arguments. Preserve measured ordinary signatures through the existing formatter. Complete decompilation is unchanged and still rejects opaque headers.

Evidence: original resource0dda43f7-e6f5-44dd-a365-8d41c0883ea9.0, Proc227 declares two parameters, first flags74306/0 and65822/0, no DefPrm; retained privately with provenance. User explicitly approved this simplification on2026-10-11. It does not claim native arity, passing modes or defaults.

The user approved generalizing the same rule to any declared parameter count after the next UNF export declared one opaque parameter. Preserve the count from the header; do not infer defaults or native passing flags.
