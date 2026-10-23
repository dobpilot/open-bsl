# Why
Original UNF inline storage contains a Structure, the acquired CatalogRef type of ВерсииРасширений, and a UUID value. The existing pure decoder preserves the latter objects opaquely; the existing storage snapshot rejects them. Original session initialization remains incomplete.

# What Changes
Add an opt-in host resolution path to the existing internal-value decoder and expose it through the closed facade. Use acquired metadata to materialize the reached catalog reference and measured UUID representation before existing storage snapshot construction. Preserve standalone opaque transit and existing snapshot semantics.

# Impact
Runtime decoder, facade, ORM storage acquisition and database adapter. No database dependency in runtime or VM, no external dependencies, no new language syntax.
