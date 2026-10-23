# Change: Preserve reached query UUID values

## Why
Original demo and UNF session initialization send the current infobase user UUID to a union query. The database adapter rejects the UUID parameter and cannot materialize a UUID result.

## What Changes
- Marshal the existing BSL UUID through the SDK typed UUID parameter.
- Materialize a typed SQL UUID cell as the existing BSL UUID value.
- Preserve the existing reference and binary paths without adding another UUID codec.

## Impact
Database query adapter in bsl-config-db, SDK integration and regression tests. No core or VM dependencies change.
