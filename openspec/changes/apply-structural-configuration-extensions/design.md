# Design

The effective graph is built once, before `SessionFactory`. Base resources and
each extension remain separate while decoded. The loader maps an
extension-local object to a borrowed base object only through the explicit base
identity recorded by the extension format; equal names are never treated as a
link. New extension objects keep their own stable identity. Active extensions
are applied in ascending platform order, so a later extension sees the graph
produced by the earlier one.

The exact serialized path carrying the borrowed-object identity must be
recorded from the three authorized databases before implementation. The local
structural survey already establishes that real extensions contain no direct
resource-name replacements: the active `demo` extension has six resources and
the two active `buh` extensions have 65 and 105, with zero names shared with
base `Config`; their serialized resources nevertheless contain references to
base identities. No private identifier or source text is retained in the
evidence.

Module changes are reduced to the existing source-neutral
`ConfigurationCodeSnapshot`: one logical module has one final source body,
visibility and owner. The loader does not rewrite BSL text and does not create
a second parser or resolver. An extension operation that cannot be represented
as a whole effective source body is an explicit error.

The ORM extension snapshot is built from acquired facts only. Identity, name,
version, hash, safe-mode settings, distributed-infobase use, activity and scope
must be present or explicitly known absent according to the upstream contract;
the adapter does not invent defaults. Inactive extensions appear in the
database and disabled collections but contribute no code. Unknown activity
still prevents construction of an effective Session graph.

`open-sdbl` owns database rows, content-addressed resources and decoding of the
generic extension envelope. `bsl-config-db` owns knowledge of which serialized
configuration objects and module slots those resources represent. `bsl-orm`
receives only provider-neutral descriptions.
