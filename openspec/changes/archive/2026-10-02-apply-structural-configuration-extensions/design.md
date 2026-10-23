# Design

On platform 8.3.27.2342 the original buh legacy-v1 extension was loaded
unchanged into a throwaway file infobase, retaining CAS generation
`efc85a95fd12b00737d88782e29d9a975342452d`. Native CheckCanApply binds
an adopted StyleItem with no target field by its class and name: renaming
the base object while keeping its UUID introduces that object's missing
error, and giving its former name to a different UUID removes the error.
A newly generated v3 header with a nil target exhibits the same behavior.
The adapter will implement this narrow measured exception. Explicit UUIDs
remain authoritative, and other object classes without targets remain
unsupported. It will not populate `extends` with an inferred identifier
or change the upstream raw-header contract.
Case-only renaming still finds the object on the platform but fails the
checked Name value comparison. Until acquisition represents that check,
the narrow fallback accepts only an exact name match; it does not mistake
folded lookup success for successful applicability.

The effective graph is built once, before `SessionFactory`. Base resources and
each extension remain separate while decoded. The loader maps an
extension-local object to a borrowed base object through its explicit base
identity, or through the narrow measured class-and-exact-name rule when the
header carries no target identity. It does not generalize this fallback to
unmeasured classes. New extension objects keep their own stable identity. Active extensions
are applied in ascending platform order, so a later extension sees the graph
produced by the earlier one.

`open-sdbl` decodes the measured object header as `belonging` plus `extends`.
For an adopted object, a non-nil `extends` is the explicit identity of the
main-configuration object. A nil link means adoption by name; only the measured
class set described below can bind by exact name. The raw header is not filled
with an inferred identity. `objects: Some(_)` means that every bare-GUID description resource
was decoded and every catalog object has its own header; `None` therefore makes
an active extension fail closed.

The local structural survey establishes that real extensions contain no direct
resource-name replacements: the active `demo` extension has six resources and
the two active `buh` extensions have 65 and 105, with zero names shared with
base `Config`. No private identifier or source text is retained in the
evidence. A later read-only `buh` check found one active extension whose object
set is incomplete and one active extension whose object set is complete. The
complete extension produced an effective code graph when exercised alone.
That initial acquisition limitation was subsequently resolved upstream; the
0.9.6 buh acquisition reaches Session compilation with both active extensions.

Module changes are reduced to the existing source-neutral
`ConfigurationCodeSnapshot`: one logical module has one final source body,
visibility and owner. The loader does not rewrite BSL text and does not create
a second parser or resolver. An extension operation that cannot be represented
as a whole effective source body is an explicit error.

The ORM extension snapshot is built from acquired facts only. Identity, name,
version, platform hash sum, safe-mode settings, distributed-infobase use,
activity and scope must be present or explicitly known absent according to the
upstream contract; the adapter does not invent defaults. The upstream
`content_hash` is a content-addressed-storage root key and is deliberately not
used as the platform `HashSum` property. Inactive extensions appear in the
database and disabled collections but contribute no code. Unknown activity
still prevents construction of an effective Session graph.

The measured upstream 0.9.5 contract supplies `runtime_version` (including a
known empty version), `hash_sum` separately from
`content_hash`, `safe_mode` and `security_profile_name`, and the confirmed
`scope_kind`. A non-empty security profile selects the named profile; a known
empty profile selects the acquired Boolean safe-mode flag. Unknown scope or
missing required properties remain errors. The upstream native fixtures under
`tests/fixtures/extension/runtime-095` preserve the differential platform
observations; no storage code is mapped to DataSeparation without a measurement.
The acquired identity is the hexadecimal physical `_IDRRef`; the native
extension UUID uses canonical field order. A separate native extension probe
confirms the existing database UUID conversion for this property, so the
adapter preserves the platform identifier rather than displaying storage order.

`open-sdbl` owns database rows, content-addressed resources and decoding of the
generic extension envelope. `bsl-config-db` owns knowledge of which serialized
configuration objects and module slots those resources represent. `bsl-orm`
receives only provider-neutral descriptions.

The sanitized live survey is recorded in
`docs/research/configuration-session-startup.md`. It demonstrates that neither
authorized active extension can be applied by matching resource names and that
serialized extension values contain references to base identities whose roles
the current acquisition API does not expose.

Native measurements in open-sdbl commit ff2855d additionally cover
CommonPicture, Language, DefinedType, CommonCommand, Catalog and Enum.
The original binary buh extension binds each measured representative by name:
renaming the same UUID loses the target, another UUID with the original name
restores it, and a case-only change fails the checked Name property.
Evidence is retained in open-sdbl tests/fixtures/extension/name-adoption-095.
The adapter accepts exact class-and-name matches for these measured classes
only; explicit target identifiers remain authoritative.

The final projection audit records two real buh extension descriptions compared
with BSL properties, including UUID checked with the existing reverse database
UUID conversion. Eight missing-field cases are covered through the extracted
private field conversion; SDK `AcquiredExtension` remains non-exhaustive and no
consumer-only constructor or duplicated SDK model is added. The failed attempt
to repeat whole-catalog construction for each negative case timed out and is
not counted as passed. Evidence and limits are recorded in
`docs/plans/configuration-extension-projection-test-gap.md`.
