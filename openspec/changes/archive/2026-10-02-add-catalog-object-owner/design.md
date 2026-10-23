# Design

Reuse ConfigDescriptor.catalog_owners; add an immutable optional owner-ID collection to the existing MetadataObject, with builder and accessor. None stays unknown, Some(empty) stays known unowned. The DB adapter preserves the acquired list without another parser. Resolve owner IDs against the existing catalog, requiring catalog objects rather than fabricating a type.

Add Owner/Владелец to the existing catalog object property table and therefore to its receiver declaration. Store the current value in that object's memory. Initialize lazily when Owner is read or written so existing callers with unknown owner acquisition can still construct objects and use unrelated properties. Unknown owner acquisition must fail explicitly only when the property is accessed.

The native single-owner and multiple-owner probes establish initial empty reference versus Undefined, invalid-string conversion, accepted owner references, body-time read and rejected body-time write, and unowned read/assignment. Reuse existing configuration-reference matching and empty-reference construction; no new global name, thread storage, connection or VM instruction is needed. Persisted-object reads, database writes and subordination rules for folders are outside these probes and remain unfinished runtime obligations.
