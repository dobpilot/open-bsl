# Change: Index query selections by column name

## Why
Native query selections accept column names in brackets. The ORM currently rejects every string index.

## What Changes
Resolve string indices through the existing case-insensitive column lookup, retaining numeric indexing.

## Impact
QuerySelectionValue and focused tests only. No dependencies or VM changes.
