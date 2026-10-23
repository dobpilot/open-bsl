# Connect reached session transactions

## Why
Original demo startup reaches BeginTransaction and fails because DatabaseDataAccess still uses the unsupported default. The prepared SDK now supplies real connection-owned transaction primitives.

## What Changes
Delegate the existing ORM transaction calls to the same BlockingDatabaseSession used for reads. Preserve the native measured nested transaction lifecycle, including cancelled reads after an inner rollback and outer completion without preserving changes.

## Impact
Only the database Session adapter changes. No new BSL API, dependencies, resolver behavior, or standalone runtime behavior. Development uses the isolated prepared SDK until its integration is available.
