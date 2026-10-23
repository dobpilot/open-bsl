# Design

Use the existing SerializedValueGraph and its RuntimeShapes type registry.
Do not create a second serializer or transfer Rc objects between threads.
ObjectProtocol exposes optional borrowed immutable snapshot bytes; only an
explicitly registered owned value type with a trusted decoder can be captured.
The graph records a declared lookup name, exact package/identity and owned bytes.
The receiving registry must match package/identity before invoking that decoder;
a same-name type from another configuration must fail, not reinterpret a UUID.
Budget accounting charges metadata, payload and auxiliary storage before copying.
measure and capture share one path and must report matching costs.

The proposed ValueTypeDescriptor builder accepts an Arc of a Send + Sync
materializer receiving bytes and returning RtResult<Value>. The builder is not
a BSL constructor, executable code is never stored in the graph, and the decoder
must validate the exact payload length. Its host-trusted registration has the
same ownership boundary as the engine's component registry.

Avoid an Arc cycle: MetadataCatalog retains original type descriptions without
decoders. Engine registration creates descriptions with decoders capturing the
immutable catalog, which contains only the originals. Reference type equality
remains package/identity equality. No decoder captures SessionData, connections,
rights snapshots or mutable object fields. Reconstructed workers reuse this
immutable engine recipe and their own host environment.

UUID values use a dedicated fixed-width graph node and the existing BslObject
UUID representation. Catalog references use exactly sixteen canonical UUID bytes,
not SQL RRRef byte order and not a diagnostic/presentation string.

Native evidence is tests/conformance/measure/1c/
measure-catalog-reference-storage.platform.txt: non-nil and empty references and
UUID values survive ValueStorage, with reference type, equality and UUID intact.
Existing JOB.PARAMS.SNAPSHOT evidence/policy governs collection copying; this
change adds immutable leaves and must not silently alter that policy.
