# Transfer immutable configuration reference values

## Why
Catalog reference parameters and session parameters must retain their identity
when captured by the existing value graph. ValueStorage currently rejects these
references, and even native UUID values are not admitted by the graph. Native
1C 8.3.27.2342 accepts both (measure-catalog-reference-storage.platform.txt).

## What Changes
- Admit the existing native UUID value to the graph as sixteen canonical bytes.
- Allow a host-declared immutable value type to explicitly supply a trusted
  materializer and its object to supply a bounded immutable byte snapshot.
- Register the catalog reference codec through the facade and retain the same
  immutable declarations in worker engine recipes.
- Keep mutable configuration objects and component objects without a codec
  rejected; no connection, principal or session container enters the graph.

## Impact
Public additive Rust API in the object protocol and owned value type declaration;
existing graph consumers and the ORM reference registration. No new dependencies
or database awareness in bsl-rt/bsl-vm. Depends on add-configuration-reference-values.
