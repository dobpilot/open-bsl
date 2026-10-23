# Design

Native 1C 8.3.27.2342 measure-dangling-enum-filling.bsl confirms that metadata returns a nonempty filled enumeration reference for an absent UUID; initial document value is empty, and Fill transfers the absent value. Preserve the seven-line oracle unchanged.

The existing constructor conflates two different checks: owner type validity and current membership of the value list. Keep owner validation and payload UUID validation; remove value-list membership as a condition of reference materialization. Named metadata value lookup remains finite. Graph restoration must retain the same identity.

Known value synonyms and empty-reference formatting stay unchanged. Missing references must not become empty strings. Native text contains a numerical storage/type code and physical UUID bytes; the isolated probe shows 54, but that number must not be copied to another database. SDK MetadataObject.number already acquires the main DBNames code. Measure its relation to native missing-reference presentation before choosing the projection and formatting contract. Unknown formatting context must fail explicitly rather than fabricate a storage code.

No production change is implemented before strict validation and the remaining presentation evidence. Existing graph and reference UUID helpers must be reused.

The supplementary native storage probe confirms `_Enum54` for the same reference printed with code 54. Use the acquired SDK DBNames number, not a hardcoded constant or a per-session counter. The existing UUID physical-byte helper matches the native payload. Localized missing-object text remains to be measured before implementation.

The same configuration and English ScriptVariant were replayed with native Enterprise /Lru. Only the missing-object text changed to Russian; the physical identity and DBNames code remained unchanged. Thus formatting must use the current session UI language, not mutate shared catalog state or infer language from ScriptVariant. ObjectProtocol.display currently has no context and references can be acquired before a Session exists. Inspect the existing formatting boundary and specify any public API change before implementation.

## Formatting boundary

The actual tree already has ObjectProtocol::presentation() -> RtResult<String>, separate from debug display, and bsl-format uses it for extension objects. Keep that existing API and add an additive presentation_with_context(&CallContext) defaulting to presentation(). Components with no contextual formatting retain their behavior. CallContext::format_value must use the contextual presentation for extension values, while primitive values continue through its existing formatter. VM String, Format and Message must route extension values through a context built from the current InterpreterServices, not capture a session in the shared materializer. Specify and verify the direct facade component-context path too.

SessionData should carry its own immutable optional system-language setting supplied when the host builds that State. Missing contextual language or acquired storage number must produce an explicit presentation error; reference acquisition, graph capture and equality must remain possible before a user session exists. No guessed code, cached language, thread-local state or database reads inside display.

Before implementation, inspect all user-visible formatting call sites so nested formatting paths cannot silently fall back to empty debug text. This change does not justify altering primitive localization or the builtin dispatcher hot path.

The additive hook and CallContext formatting route are implemented; a facade-only test verifies two State host-data values, repeated isolation, missing-context failure and retained legacy presentation. VM builtin and concatenation routing remain unfinished. Inspection found add_op and add_const_op also call formatting without HostIo, so their contextual route must be included before claiming user-visible formatting complete.

VM String, Format and Message now use the shared contextual helper for extension values. Add/AddConst string conversion uses the same helper; primitive formatting remains on bsl-format. The facade contextual test passes with both default and all optimizations, including string accumulation into its own register. Original arithmetic and coercion rules are retained. Full corpus and workspace gates remain required before finalization.

Expose the existing runtime uuid module through open-bsl so ORM reuses canonical parsing and formatting. Preserve declared synthetic identities used by existing component catalogs; require canonical UUID syntax for identities absent from the declared list. Canonicalize absent UUID case before equality and graph storage.

MetadataObject receives an optional reference_storage_number builder/getter populated from the acquired main DBNames code. SessionData receives an optional immutable system_language builder/getter propagated from SessionResources. Lift the existing physical-to-canonical UUID permutation from the database adapter into the runtime uuid module and add its inverse there; the facade exposes both through its uuid module. This keeps byte ordering shared without linking ORM to the database SDK.
