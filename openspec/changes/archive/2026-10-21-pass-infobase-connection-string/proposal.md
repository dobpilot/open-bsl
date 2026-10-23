# Pass the information-base registration string from CLI

## Why
Original demo startup reaches the existing Session connection-string function after its database queries, but CLI cannot provide the host value to SessionFactory.

## What Changes
Add optional --infobase-connection-string for --config-db launches. Forward its exact value through the existing SessionFactory builder. Keep standalone execution and omission behavior unchanged.

## Impact
Only CLI argument plumbing and help change. No new BSL or facade API, inferred cluster address, registration-file reader, or database dependency.
