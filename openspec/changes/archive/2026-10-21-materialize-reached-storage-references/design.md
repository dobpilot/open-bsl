# Design
The read-only original receipt unf-storage-type-1013-startup.log reports Structure 4238019d-7e49-4fc9-91db-b6b951d5cf8e, CatalogRef a7a5262c-5d85-4e39-b132-c8f52dac93cb, and UUID fc01b5df-97fe-449b-83d4-218a090e681e. A separate metadata-only receipt identifies exactly one owner: ВерсииРасширений, 4bb0f7c3-62f3-4352-9bc8-e243dd18fe4a. No original payload is retained.

Extend the existing parser with an explicitly supplied resolver for unknown scalar object representations. The resolver receives the parsed type UUID and scalar payload, not an independently parsed raw format. Collections keep the existing parser and identity cache. Default entry points retain opaque transit. Resolver errors propagate; unresolved objects retain the existing explicit unsupported-storage failure.

The database adapter knows acquired platform type UUIDs and maps the reached CatalogRef to the existing ORM reference constructor. Runtime and VM do not know configuration metadata or database providers. UUID restoration uses the existing UUID parser. Existing engine reference materializers and graph snapshot rules remain authoritative. The exact scalar framing must be measured before implementation.

# Native scalar evidence
Own platform control measure-storage-reference-serialization.platform.txt completed with five lines: canonical UUID scalar; CatalogRef scalar 53:abcd0123456789ab4321123412345678 for UUID12345678-1234-4321-abcd-0123456789ab; nil reference scalar53:32zeroes; Get preserves reference and UUID equality. The numeric prefix meaning is not inferred from this control. Reuse existing uuid::from_1c_bytes for the measured byte permutation; do not add a second codec.

The verified-source prefix control preserves reference equality and type for numeric prefixes 0,8,53,54,999999,-1,4294967296. Empty and text prefixes produce native errors. Thus the prefix must not be compared to an acquired table number: the type UUID remains the owner key. Additional numeric spelling control is running; the prototype digit-only prefix check is not the final accepted grammar.

Final native spelling control also accepts +1 and 00053, rejects 1.5 and 1e2. The prototype now accepts optional sign and decimal digits, rejects empty/nonintegral forms, and uses only acquired type UUID to select the owner.
