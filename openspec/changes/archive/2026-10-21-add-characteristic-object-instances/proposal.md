# Change: Add characteristic object instances

## Why
The original demo manager module calls `СоздатьЭлемент` for a chart of characteristic types. Its method context is currently empty, so factory compilation fails. Native help and independent platform receipts now establish creation and object-module initialization behavior.

## What Changes
- Carry acquired characteristic settings and configured value types in the ORM metadata catalog.
- Register characteristic object and reference types through the facade.
- Add real in-memory creation and measured standard properties, retaining per-instance object-module memory.
- Connect manager and object contexts through existing open-bsl extension points.

## Impact
Only bsl-orm, bsl-session, the database projection and consumer tests are affected. Config decoding remains in open-sdbl; missing SDK data must remain explicit. This does not implement database writes or remove unsupported protected modules.

## Qualified value type descriptions
Preserve acquired characteristic qualifiers with an additive runtime qualified type description value and a facade-visible `BslValue::new_qualified_type_description(Vec<ColumnType>)` factory. Cover conversion, ORM assignment/reset, internal text round trips and typed column input with independent native receipts. Preserve the existing unqualified constructor and graph-transfer behavior.
