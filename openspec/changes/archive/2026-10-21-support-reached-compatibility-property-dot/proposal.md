# Support reached compatibility property member access

## Why
Original demo reaches Metadata.ObjectProperties.CompatibilityMode.DontUse. The runtime explicitly rejects dot access for this enumeration while supporting indexed access. A direct native control confirms both access forms and language aliases.

## What Changes
Use the existing member lookup for dot access to the compatibility property enumeration. Keep its existing type, display, index behavior and namespace restrictions.

## Impact
One runtime rejection branch is removed. No global name, resolver priority, dependency or old test expectation changes.
