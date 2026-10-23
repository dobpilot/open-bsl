# Change: Ignore unreferenced BSL query parameters

## Why
Original demo retains an unused parameter on a query object; the strict SDK rejects it. Native 1C accepts unused UUID, table and retained parameters while still rejecting a missing used parameter.

## What Changes
Pass only parameters referenced by the current BSL query text to the strict SDK. Keep the query object's parameter collection unchanged.

## Impact
Private implementation and tests of the bsl-config-db query adapter. No SDK compatibility or language resolver changes.
