# Design

Program name tables are immutable once assembled. Use std Arc<[String]> rather than a custom copy-on-write container or a new lookup mechanism. The compiler retains its existing mutable NameInterner and freezes its result once after catalog chunk compilation. Each module and optional same-compilation entry clones the Arc, preserving exact order, spelling and NameIds.

Where execution needs to append names, keep the existing mutable RuntimeShapes/interner path and explicitly copy into its owned vector. Do not mutate shared program tables or change dynamic prefix rules. Standalone assembly freezes its own table.

Text remains format 44 with the same table contents and printing. Image parsing may share a module table only after content equality is established; it must not replace a different table or mask an existing verifier error. Parser equality/sharing and compiled-table sharing need focused tests. Shape storage is unchanged.

The baseline diagnostic counts name-table String storage and distinct array allocations; it excludes allocator overhead, AST, IR, shapes and all other RSS. Do not describe it as whole-process memory. Use controlled A/B for numerical performance claims and repeat original UNF under observed memory bounds after correctness checks. Check the VM machine-code screen if representation changes affect hot-path layout.

Existing test expectations must remain unchanged. If raw Vec field literals or deliberate test mutations require mechanical adaptation, prepare the exact patch and obtain the repository-required approval before applying it. No broad test rewrite or serialized format bump is justified by ownership alone.
