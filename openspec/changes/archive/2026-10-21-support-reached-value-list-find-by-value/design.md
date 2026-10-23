# Design

Scan the existing item vector in current order, compare the item value using existing BslValue equality and clone the selected item handle. Do not create a replacement item. Reuse the existing ValueListItemObject access helper and component method table.

The direct native control in docs/research/fixtures/value-list-find-by-value establishes empty/missing lookup, first duplicate and insertion order, numeric equality, separate scalar types, case-sensitive strings, Undefined/Null and same-versus-distinct arrays. Syntax Assistant FindByValue189.html confirms one required arbitrary value argument and an item-or-Undefined result.

Verify primitive cases in external runtime tests and execute the complete retained native script through CLI after regeneration of the API reference. Existing tests must remain unchanged.
