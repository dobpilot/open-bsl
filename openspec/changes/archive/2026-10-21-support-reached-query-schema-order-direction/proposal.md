# Why
Original BUH compilation reaches QuerySchemaOrderDirection.Ascending in the ContactPersons manager module. Existing runtime tables include Descending but omit Ascending. A scan of original BUH sources also reaches Descending.

# What Changes
Complete the two reached members through existing runtime enumeration and type tables, with native Russian and English names and presentations. No query schema object model is introduced.

# Impact
Runtime system enumeration tables, generated API reference and focused native regression. No dependencies, opcodes or resolver rules change.
