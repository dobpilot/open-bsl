# Change: Apply structural configuration extensions

## Why

The database loader currently accepts only an active extension whose every
resource replaces an existing numbered base resource. Real `demo` and `buh`
test databases carry active structural extensions: their resources use an
extension-local identity space, add objects, and refer to borrowed base
objects from inside serialized metadata. The loader therefore rejects the
configuration before a Session can be compiled.

## What Changes

- Decode the extension-local configuration catalog and its links to borrowed
  base objects without exposing storage details above `bsl-config-db`.
- Apply active extensions in platform order into one effective module graph,
  including new modules and changed modules.
- Project the complete acquired extension description into the immutable ORM
  catalog instead of rejecting every non-empty extension set.
- Keep inactive extensions out of the effective code graph while retaining
  them in the database extension snapshot.
- Fail closed on an unknown activity, an unsupported merge operation, or an
  incomplete extension description.

## Impact

- Affected specs: `configuration-code-loading`, `configuration-session`
- Affected crates: `bsl-config-db`, `bsl-orm`
- Required upstream input: `open-sdbl-db::AcquiredExtension` must retain the
  decoded catalog/root information and extension properties needed by the
  provider-neutral loader; `open-sdbl` remains unaware of BSL modules.
