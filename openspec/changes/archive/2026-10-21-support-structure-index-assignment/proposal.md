# Why
Original demo stops with NotIndexable in ValuesOfObjectAttributes after array query parameters are supported. Its source assigns Result[FieldName]; the base runtime has no structure index mutation. Native controls confirm assignment to an existing field and indexed argument writeback.

# What Changes
Support writes to existing mutable structure fields by case-insensitive string index through the existing field storage and name interner. Preserve other indexed objects and collection traversal.

# Impact
Runtime indexed-write API, VM direct assignment and open output argument writeback. No dependencies or bytecode format changes.
