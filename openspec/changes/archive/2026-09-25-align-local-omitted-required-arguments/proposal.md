# Align explicitly omitted local arguments with 1C

## Why

The local-call resolver rejects an explicitly empty required argument slot, while platform 8.3.27 passes `Undefined` in that slot. A reached configuration call therefore fails during resolution before its session module can run. Imported configuration-module calls already have the measured behavior.

## What Changes

- Resolve an explicitly empty required slot in a local procedure or function call as `Undefined`.
- Keep an absent required trailing slot a compile error and keep defaulted parameters on their existing default path.
- Replace the existing contradictory resolver expectation only after recording the platform measurement.

## Impact

Only local-call name resolution and its tests change. The compiler, bytecode, VM, and crate dependencies remain unchanged.
