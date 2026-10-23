# Design

Use the existing BslObject::Uuid, open-bsl UUID formatter/parser and SDK Guid. The SDK owns physical storage conversion and ColumnKind::Uuid. The adapter accepts Cell::Uuid only for a UUID result column; mismatched cells still fail. Catalog references retain their independent physical-byte path. No thread state, binary-as-UUID guess, new dependency or new BSL type is introduced.

Evidence is retained in docs/research/fixtures/uuid-query-parameter-native-1015: native 8.3.27.2342 writes a UUID attribute, matches an equal parameter, rejects a different parameter and returns UUID. Typed SDK projection, subquery and union controls are executed against that same own database before original read-only retries. No original infobase is opened through the native client.

Changes are first verified in the existing isolated integration because the main SDK checkout does not yet contain its prepared APIs. Integration and original terminal results remain explicit tasks.
