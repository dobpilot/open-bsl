# Design

## Evidence

See docs/research/fixtures/buh-configuration-name-limit-1016. Both modules compile independently; their combined catalog fails only when a new method follows 65536 property names. A separate VM reproducer demonstrates that distinct name tables can confuse fields on structures passed between programs. Per-module name namespaces therefore cannot be substituted without additional translation and are outside this change.

## Proposed implementation and gates

Keep the current path for every catalog that already compiles. On name-index exhaustion, retry with a fresh common interner and fresh shape table, placing statically required narrow-operand names before property-only names. Collect required names from independently generated module chunks using the existing configuration-method rules, plus the flat resolved component-link table; do not assemble these diagnostic chunks or invoke `compile_links`. Merge all local name tables in module order before selecting narrow names, so a property encountered before a differently cased method preserves its spelling. This avoids a duplicate recursive traversal of the resolved syntax. Include both open call operands and any narrow object-property operands. It must cover open object calls and both type/method names in component links, and preserve the first source spelling of each case-insensitive name. A catalog requiring more narrow names than the existing operand range must return a contextual compilation error rather than panic or truncate.

No instruction, public type, VM dispatch, serialized format or dependency change is proposed. NameIds may be reordered only for a catalog that previously failed compilation; all modules and its optional entry must receive the same final table and compatible shapes. Existing successful table order is preserved.

## Limits

Dynamic snippets and notification handlers currently also use narrow method operands. This change must preserve their catalog prefixes and existing behavior; it does not claim to remove every dynamic name-capacity limit. A module that already exceeds its own name-index limit remains an explicit failure; the scope is inter-module exhaustion. Actual BUH narrow-name cardinality and runtime controls must be checked before treating the proposed retry as sufficient.

This is an implementation-capacity change, not a new decision about 1C language semantics. Existing measured behavior remains the execution oracle.

## Text parser correction discovered by the round trip

The new large-table round-trip control fails because `field_name` parses a `NameId` with `field_u16` although `NameId`, GetProp and SetProp already store u32. Read this existing field with `field_u32`; retain the same decimal syntax, printer, instructions and format version. Narrow method operands retain their own u16 parsing and checks. This restores round trips for representable property indices rather than introducing a new encoding.
