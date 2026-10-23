# Design
The reached UNF field has Binary(unknown length), 410 bytes, prefix 01 01 and no STORHDR external reference. SDK decode_stored_value recognizes an internal serialized list. Evidence comes from the original read-only run, not a native round-trip of arbitrary invented storage data.

Reuse SDK stored-value framing and bounded inflation. Extend its decoder with a text entry point shared by the existing tree decoder rather than copy its ten-byte header handling into the adapter or reserialize metadata trees. Reuse the runtime vstr decoder through a pure facade wrapper and existing ValueStorage snapshot construction. The existing constructor's detached graph semantics stay intact.

Recognize ValueStorage from the acquired field declaration and compiler column origin, using the existing platform type mapping (e199ca70-93cf-46ce-a54b-6edc88c3a296). Never infer a storage type solely from arbitrary Binary bytes. Rights and mandatory data separation remain prerequisites to reading. Unknown declarations, external STORHDR references, unsupported serialized objects, malformed and oversized values fail explicitly.

This change does not claim complete stored-value database support or change BSL syntax. Independent native storage controls and the original UNF replay must verify the reached path before archival.
