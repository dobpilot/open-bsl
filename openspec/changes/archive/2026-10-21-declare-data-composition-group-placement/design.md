# Design
Reuse the existing EnumKind, EnumValue and TypeId machinery. The installed vendor page and native 8.3.27.2342 control confirm End, Begin, BeginAndEnd and None; runtime strings are End, Begin, Begin and end, None. The member type prints DataCompositionGroupPlacement. Both the bare enumeration and its meta-type print EnumDataCompositionGroupPlacement, confirmed in a second native control. See docs/research/fixtures/data-composition-group-placement-1015.

Only enum declaration is in scope. Original BUH currently fails during graph compilation; this is not evidence that report construction executes during session initialization. Existing variable resolution priority remains unchanged.


## Reached text placement enum
The same report manager next reads DataCompositionTextPlacementType.Wrap. Keep the two related placement-enum declarations in this unarchived change. The full vendor page lists Overflow, Block, Cut and Wrap; it does not list Auto. Native controls confirm aliases, member and meta-type presentations, distinct identity and rejection of Auto. Block prints Conceal and Cut prints Clip. See docs/research/fixtures/data-composition-text-placement-1015. Retain the native order in the member table; adding iteration machinery is out of scope. This is distinct from SpreadsheetDocumentTextPlacementType; do not reuse that enum's identity or Auto member.

Native Type() lookup accepts Russian and English names for each newly declared member type while printing its English name. The lookup names belong in NAMES; the measured presentation remains in DISPLAY. The shared native receipt is in docs/research/fixtures/data-composition-settings-item-state-1015.
