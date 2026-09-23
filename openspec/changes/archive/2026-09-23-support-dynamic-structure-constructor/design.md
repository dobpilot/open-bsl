# Design

The resolver keeps the existing `NewStructure` representation when the first argument is a string literal. That path interns a finite compile-time shape and retains its current exact value-count check. A non-literal first argument becomes a separate `NewDynamicStructure` expression containing every constructor argument in source order.

The compiler evaluates those arguments from left to right into consecutive registers and emits a new `NewDynamicStructure` instruction appended to the bytecode enum. The textual bytecode format represents its destination, base register, and argument count and increments the format version. The shared instruction-effects table remains the only dependency analysis source.

The VM delegates construction to the base runtime. A fixed-structure first value is copied when it is the sole argument. Every other first value uses the same user-visible conversion as `Строка`; the resulting comma-separated names are trimmed and empty names are skipped. Values are assigned in field order, absent values become `Неопределено`, and already evaluated excess values are ignored, as recorded by `measure-dynamic-structure-constructor.bsl`.

Runtime field lists must not enter `ShapeTable`: a loop can produce unbounded distinct lists, and interned shapes live for the whole execution. The base runtime therefore creates `StructureStorage::Dictionary` directly while preserving insertion order and folded field lookup. Literal constructors continue to use shaped storage and their existing inline-cache path.
