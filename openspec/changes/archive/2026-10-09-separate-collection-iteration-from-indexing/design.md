# Design

Native evidence: `tests/conformance/measure/1c/measure-common-attribute-content-surface.bsl` and its 25-line platform receipt on 8.3.27.2342. Count and For Each succeed, numeric brackets and Get fail, and content-item properties are read-only. The earlier control without the conditional constant had incomplete setup and is not the retained oracle.

Introduce `ObjectProtocol::collection_item(&Value)` with a default delegation to `get_index`. Keep the current count-once/index-increment For Each topology: the change separates the item retrieval operation, not the iteration state machine. Non-extension values continue through their existing GetIndex path. This preserves deliberate map/index behavior and mutation handling already accepted in open-bsl.

Add one `GetCollectionItem` instruction with the same register operands as GetIndex; ordinary indexed expressions continue to emit GetIndex. Classify the new instruction in the existing exhaustive effects table, and use that table for verification and optimization as before. Dispatch the extension callback in the existing VM path. No extra per-iteration allocation or speculative iterator abstraction is required.

Extend the common textual printer/parser/opcode table and round-trip corpus together, and bump the format version to 44 for the new encoded operation. Existing tests that intentionally pin the current format require a concrete reviewed expectation patch under the user's regression rule; prepare it before requesting approval. Do not weaken image validation or duplicate the instruction-effects table.

All existing implementations inherit the default behavior. A host collection can override collection_item while retaining get_index's rejection. The facade already exports ObjectProtocol; an external consumer test must prove no internal crate name is needed.
