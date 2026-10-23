# Design

Full vendor help `platform-syntax-help:syntax-context:objects/catalog125/catalog126/object130/properties/IsFolder1613.html` describes a read-only boolean standard catalog-object property.

Extend the existing finite property table and receiver context; do not add a system builtin or a parallel resolver. Measure the already supported newly created item, including its object-module body and assignment attempts, before runtime code. Do not claim a persisted record or a folder constructor exists merely because the property is declared.

The flag belongs to object state. Future record acquisition and folder creation must supply their actual flag through the same object representation, rather than using an unrelated singleton or a second object model. No new dependency or database write is required to represent a new item's measured state.

Native 8.3.27.2342 produced eight rows in `measure-catalog-folder-flag.platform.txt`: a new item reads false already in the object-module body, body and later assignment are rejected, bare/qualified access returns false and the English alias has boolean type. Add an immutable per-object flag initialized false by the existing CreateItem path; use the same property table for aliases and readonly rejection.
