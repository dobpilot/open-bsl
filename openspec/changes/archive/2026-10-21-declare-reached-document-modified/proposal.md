# Why
Original BUH compilation reaches bare Modified() in Document.RegulatedOperation.ObjectModule. The existing document receiver table lacks its measured zero-argument signature.

# What Changes
Declare Modified/Модифицированность in the existing document-object receiver table. Keep runtime calls explicitly unsupported until document mutation tracking exists. Preserve local function precedence and standalone undefined-name diagnostics.

# Impact
ORM method table, generated API reference and focused Session checks. No document persistence, mutation-state emulation, resolver changes or new dependencies.
