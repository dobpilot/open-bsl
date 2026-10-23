# Design
Use the existing DocumentObject MethodDescriptor table and receiver context projection. Append a zero-argument method with Russian name first and English alias Modified. Its callback reports that document modification-state tracking is not connected. It does not return an invented Boolean. Object-module bare calls follow the existing owner receiver path; local functions retain core precedence. No global function is registered.

Vendor page Modified305, platform8.3.27.2342, and the own native document control confirm zero arguments, Boolean result type, English alias, bare object context and rejection of one argument. Evidence: docs/research/fixtures/document-modified-native-1015. This change declares the reached signature only; native change-state tracking is intentionally not implemented. If original initialization actually calls the method, that explicit failure becomes the next implementation requirement.

Preserve the existing document dispatch convention used by Fill/Write/Copy: a same-named existing module export retains its own direct-call arity and result. Extend only the explicit dispatch allowlist for the new aliases; generic resolver priority remains unchanged.
