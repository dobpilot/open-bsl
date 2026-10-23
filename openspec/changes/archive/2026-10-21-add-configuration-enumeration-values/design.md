# Design

## Evidence
Platform 8.3.27.2342 probes live beside `measure-configuration-enum-reference.bsl`. Russian-script and Russian-default-language variants isolate independent configuration settings. Changing the Enterprise interface language alone does not change the measured enumeration presentation or full name. Russian ScriptVariant changes `Enum` to `Перечисление`; a Russian default language selects the acquired Russian synonym instead of the member name.

## Implementation boundary
Reuse the existing immutable host-declared ValueType and graph materializer machinery. Enumeration values remain separate from catalog/document references: the measured enumeration reference rejects UUID retrieval, so sharing their public method table would create incorrect behavior. Cache type declarations once per configuration snapshot, not per session. The materializer carries only owner/member identity and validates it against the receiving catalog; it does not carry principals, connections or session containers.

Acquire presentation from existing metadata descriptors rather than guessing from a member name. A missing synonym falls back to the member name as observed in the English-default-language probe. Full-name formatting uses the acquired ScriptVariant. Existing host descriptors with no presentation keep their current name fallback. Preserve existing local/module/global resolution decisions.

## Isolation
Background jobs receive immutable values through the existing graph and restore them into their own State. This adds no host-data propagation and changes no background session connection ownership. General language fallback rules and unmeasured manager error cases require further platform probes before implementation decisions.

## Numeric index conversion
Native manager probes establish string/Boolean numeric conversion and truncation toward zero for Get. Re-export the existing `bsl-format` number parser and its format descriptor through the facade so ORM can reuse the same parser without depending on an implementation crate or duplicating it. Invalid and out-of-range inputs remain failures. Native IndexOf rejects empty and foreign values.

The parser facade wrapper maps the existing NumError into the already exposed RtError. NumberFormat and its Locale field type are both reachable through open-bsl. A consumer test explicitly names the format, locale, numeric result and error using only facade imports.
