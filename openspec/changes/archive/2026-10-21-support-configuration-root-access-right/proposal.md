# Change: Check reached configuration root rights

## Why
Original UNF reads SaveUserData on Metadata, and original user initialization checks Administration on the same root. The ORM rejects MetadataRoot and the database adapter has no rights implementation for it.

## What Changes
Represent the acquired root as MetadataKind.Configuration in existing access requests. Delegate root checks, including privileged frames, to the existing rights-aware adapter. Support measured SaveUserData and Administration with current, role and user subjects and native administrative checks.

## Impact
ORM request construction and documentation, database adapter role evaluation, focused native-backed tests. No database dependency in the runtime or VM and no new external dependencies.
