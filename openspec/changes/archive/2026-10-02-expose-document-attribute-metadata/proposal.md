# Expose acquired document attribute metadata

## Why
The native missing-enumeration probe cannot replay its metadata-read block because document metadata lacks Attributes. The catalog already carries acquired immutable attribute identities, names, filling flags and serialized filling values; no additional database API or decoding is needed for this projection.

## What Changes
Expose document Реквизиты/Attributes, name and numeric indexing, Count, Get, Find, and read-only attribute Name, FillFromFillingValue and FillValue through the existing metadata component protocols. Unknown acquired collections or filling data fail explicitly instead of becoming empty defaults. Replay native metadata and complete missing-reference probes.

## Impact
Public BSL metadata properties and collection method Get, generated API reference, ORM runtime and Session integration tests. No new dependencies, UUID parser, database reads, source rewriting or resolver changes. Common-attribute acquisition and catalog object owner support remain separate unfinished tasks.
