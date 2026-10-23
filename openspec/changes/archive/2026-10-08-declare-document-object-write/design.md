# Design

Reuse the existing finite document receiver method table and the catalog Write declaration pattern. Syntax help 8.3.27.2342, document `platform-syntax-help:syntax-context:objects/catalog125/catalog143/object146/methods/Write303.html`, confirms two optional parameters: DocumentWriteMode and DocumentPostingMode. Defaults are Write and NonRealtime; this change does not implement their effects.

The original UNF source has bare calls at lines 165 and 216 of Documents/Событие/Ext/ObjectModule.bsl. Declaration lets uncalled branches compile. Calling the platform method fails explicitly and leaves the object unchanged; it must not invoke write handlers, allocate numbers, change selection state, or write rows. That failure is a deliberate implementation limit, not an inferred platform behavior.

Preserve existing exported Write/Записать methods before the new platform declaration, using the existing Fill export check. Do not alter priority for other methods. Persistence, transaction scope, rights, RLS, event ordering and record movements require separate measured implementation.

## Verification receipt

All 24 pre-state-check document-object tests passed, and the additional failure-state test passed. The complete workspace gate passed on 2026-10-02 (tests 347.2 seconds), including rustdoc, facade consumer, normal dependency trees and all strict OpenSpec validation. Log: /tmp/open-bsl-document-write-096-gates.log. Original UNF thin startup failed after 162.8 seconds at the next missing builtin DaylightTimeOffset, with no user-script marker. Actual document persistence remains unimplemented.
