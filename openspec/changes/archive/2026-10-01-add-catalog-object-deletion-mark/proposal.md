# Expose a new catalog item's deletion mark

## Why

Original UNF Catalog.КатегорииНоменклатуры.ObjectModule reads bare ПометкаУдаления. The existing catalog-object receiver does not declare this standard property and session compilation fails.

## What Changes

Measure native reads, assignment and conversion, then expose ПометкаУдаления / DeletionMark through the existing per-object state and property table. Preserve ordinary variable priority. This property change does not implement record writes or the separate SetDeletionMark method.
