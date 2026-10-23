# Design

The installed 8.3.27.2342 Syntax Assistant documents GetTemplate with one
required argument (String or template metadata). Full page:
`platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/GetTemplate362.html`.

Use the existing information register MethodDescriptor table, as for catalog,
document and data processor template methods. A bare call resolves only in
the owning manager receiver context. Existing exported manager methods retain
their arity and result; no global unresolved-name fallback is added.

Actual template acquisition is unfinished. This change supplies the accurate
method surface and an explicit unsupported error, not an empty template.
Real retrieval remains necessary if original startup executes this method.

The catalog, document and data processor template test files were created
during this same configuration runtime task and are not baseline tracked tests.
Their negative assertions for information register template availability must
now cover other manager kinds; baseline regression expectations remain unchanged.
