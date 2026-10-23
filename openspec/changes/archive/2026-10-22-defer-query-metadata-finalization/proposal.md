# Defer query metadata finalization

## Why
Opening a configured Session repeatedly creates complete query snapshots while
materializing ORM metadata. Measured UNF startup performs 17 resolves totaling
3.95 seconds; the query fingerprint alone accounts for 6.8% of sampled cycles.
ORM metadata access must not pay for query-only finalization.

## What Changes
Separate normalized runtime metadata facts from complete immutable query
snapshots. ORM materialization uses the former. Actual query preparation or
explicit snapshot acquisition continues to obtain a fully validated snapshot
with the existing fingerprint and query identity guarantees.

## Impact
The database provider consumes the SDK runtime-facts API. No new dependencies,
BSL surface, or changes to name resolution, authorization or session isolation.
