# Design

Reuse MetadataKind::receiver_methods and the existing platform-first dispatch. Add no resolver exceptions or VM machinery. These are information-register methods only, not catalog methods. A method that is known but unavailable raises a named runtime error; no empty record or successful write is fabricated.

Signatures are confirmed by native syntax help 8.3.27.2342 (Russian), vendor documents:
- platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/CreateRecordManager324.html
- platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/CreateRecordSet355.html

Both factories take no arguments. CreateRecordManager returns an information register record manager; CreateRecordSet returns an empty record set. No platform-value behavior is inferred by this declaration. The explicit unavailable error is an implementation boundary, not claimed platform semantics.
