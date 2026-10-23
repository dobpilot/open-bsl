# Report ordinary SDK session mode

## Why
Original demo startup reads ExclusiveMode. The host explicitly opens an ordinary SDK session without platform exclusive ownership.

## What Changes
Allow this adapter to report false for that session. Changing exclusive mode remains unsupported.

## Impact
Only the database adapter; no platform locking implementation or resolver changes.
