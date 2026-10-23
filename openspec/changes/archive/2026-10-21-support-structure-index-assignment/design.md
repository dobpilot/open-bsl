# Design
Use the existing NameInterner lookup and StructureStorage field mutation. The existing context-free set_index remains the path for objects that do not require names. Add a documented indexed-write entry that receives names, and use it for VM assignment and output argument writeback. Do not add a name table to each structure or alter iteration.

Native evidence is retained under docs/research/fixtures/structure-index-write-native-1015. Native 1C 8.3.27.2342 confirms mixed-case assignment without insertion, rejection of absent fields and numeric indices, and immutable FixedStructure writes. Full original replay must identify the failing instruction before this change is called a fix of that replay.

The native control also exposed an existing unrelated static-procedure indexed-reference argument divergence. Static calls currently transport indexed expressions by value; this change does not redesign that call contract. Its output-argument scope is the existing Structure.Property protocol. The additional native source is retained for a separate task.
