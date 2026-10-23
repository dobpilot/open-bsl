# Design

Map the SDK hierarchy enum into a dependency-free ORM enum stored optionally in MetadataObject. Keep it separate from catalog_hierarchical; SDK and native XML preserve the configured kind even when hierarchy is disabled. None is unknown and must not default to folders-and-items.

Native CreateFolder248 syntax help declares zero arguments and a catalog object result. Four native probes establish: folders-and-items with hierarchy enabled produces a folder; folders-and-items with hierarchy disabled produces a normal item; item-only kind rejects creation with either hierarchy flag. Require known settings and reject item-only before construction rather than initializing and discarding a value.

Reuse catalog_object::create for memory, receiver and module initialization. Make the existing folder flag a Cell and set it after the body, before returning the selected object. The module body continues to observe false and reject writes; CreateItem remains false. Add one row to the existing manager method table, automatically providing bare manager context as well as qualified calls. Do not add a second constructor path, resolver ordering or VM instruction.

Persistence, owner subordination of folders and folder-specific validation remain broader runtime obligations, not fabricated behavior in this change.

The new regression reproduces an existing one-argument CreateFolder manager export being replaced by the factory descriptor. Extend the manager's already existing export-preservation branch, used by FindByAttribute, GetTemplate and CreateDocument. This is the host's additive API contract, not a claim about unmeasured native same-name declarations. Folder owner assignment remains an explicit unsupported operation until subordination rules are acquired and measured.
