# Native data processor descriptors

Unchanged native 1C Config resources copied from open-sdbl's
`tests/fixtures/data-processor-0911/` control. Both repositories retain paired
Designer XML, decoded Config, CF and acquisition provenance. These bytes test
adapter projection through SDK descriptors, not a second Config decoder.

`Замеры`: ff92317d-71e7-4664-a13a-18bd31fd3701.
`ПустаяОбработка`: 4722aab2-abc1-40ce-80b4-6f540dfa1891.

The separate `order.*` pair reverses the fields of ДругаяЧасть; the adapter
test verifies that its nonalphabetic field order reaches the ORM unchanged.
