# Expose catalog object folder flag

## Why

Original UNF Catalog.КатегорииНоменклатуры.ObjectModule reads bare ЭтоГруппа. The existing catalog-object receiver has no declaration for this standard property, preventing session compilation.

## What Changes

Measure native reads and assignment rejection, then expose the read-only boolean ЭтоГруппа / IsFolder through the existing object property table. Preserve variable priority and avoid changes to the global resolver. Creating folders and reading persisted records remain parts of the overall runtime plan; this change describes the already supported CreateItem object.
