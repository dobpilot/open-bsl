# Evidence
Native controls and receipts are in `docs/research/fixtures/planner-item-constructor`. Both aliases create a new object with zero arguments. One, two and three arguments fail. Each instance starts with undefined Value/Deleted/Parent, empty Begin/End dates, empty Text/ToolTip and an empty fixed map of DimensionValues.

Assignment controls establish String conversion for numeric Text/ToolTip, replacement of a previous Begin date by an empty date when a number is assigned, and Undefined Deleted after assigning True to a standalone item. Parent and DimensionValues reject replacement. The first chained constructor/count expression fails; the corrected separate-object probe measures count zero. Do not mistake the initial grammar failure for a nonempty or inaccessible map.

# Implementation
Use one private component object with per-instance fields and existing Value representations. Implement fixed-map construction once in the runtime and expose the same factory through the facade if required; do not duplicate the fixed-map protocol in bsl-session. Match existing component descriptors and Russian-first aliases. Further property conversions require native measurements. Unimplemented planner collection, scheduling and visualization operations must not claim success.

# Verification
Use external facade tests with native receipts, preserving their localization. Validate exact values by comparisons when localization would obscure equality. Repeat compile-only demo before original execution. Existing tests are not changed without explicit approval.

The fixed-map snapshot factory lifts the existing constructor implementation unchanged; the BSL constructor delegates to it. Other text/date conversions and non-True deletion inputs currently return explicit unsupported errors rather than assuming unmeasured platform behavior. The separate native End-number probe confirms the same empty-date replacement as Begin.
