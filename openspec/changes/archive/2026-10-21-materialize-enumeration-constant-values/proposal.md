# Change: Materialize enumeration constant values

## Why
Original UNF session startup reads an enumeration constant. The adapter currently attempts to create a catalog reference and rejects its owner.

## What Changes
Reuse existing enumeration values when materializing a configuration reference owned by an enumeration. Preserve known and empty value identity.

## Impact
ORM reference construction and database adapter dispatch. No dependencies or VM changes.
