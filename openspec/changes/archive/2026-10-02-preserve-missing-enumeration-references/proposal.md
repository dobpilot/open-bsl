# Preserve missing enumeration references

## Why
Real buh metadata contains an enumeration filling value whose UUID is absent from its value list. Native 1C retains this typed reference and transfers it during document filling; the ORM currently rejects it during acquisition.

## What Changes
Accept valid enumeration identities independently of whether the catalog lists their values. Preserve UUID, type, equality, hash, filled state and graph round-trip. Define missing-value presentation from acquired storage identity and native evidence rather than silently returning an empty string.

## Impact
ORM reference materialization and formatting, acquisition integration tests and native conformance. Existing named value lookup remains strict. No new dependencies or guessed identities.
