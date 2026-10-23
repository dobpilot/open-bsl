# Execute measured balance temporal values

## Why

The accepted balance contract already requires PointInTime and inclusive/exclusive Boundary values. The host still rejects Boundary construction and only forwards Date balance arguments. Native controls now retain 60 manager/query observations and an independent Boundary object probe.

## What Changes

- Replace the temporary Boundary constructor failure with the measured immutable wrapper.
- Carry Date, PointInTime and supported Boundary values through the existing SDK parameter and logical-result contracts.
- Preserve explicit rejection outside measured Balance positions and existing principal, RLS and separation enforcement.

## Impact

Affected crates: bsl-orm and bsl-config-db; dependency version integration uses the independently reviewed SDK 0.9.28. No RT/VM dependencies or alternate BSL/SQL execution engine.
