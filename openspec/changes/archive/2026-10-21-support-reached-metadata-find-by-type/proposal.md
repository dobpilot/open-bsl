# Change: Resolve metadata by the reached reference type

## Why
Original demo now reaches Metadata.FindByType while grouping catalog references. MetadataRoot exposes no methods and raises UnknownMethod.

## What Changes
Expose НайтиПоТипу/FindByType using the immutable configuration type declarations and existing metadata object values.

## Impact
ORM root method table, focused tests and API reference. No database queries, resolver changes or new type identities.
