# Add the catalog object Parent property

## Why
Original UNF Catalog.GroupsOfUsers.ObjectModule cannot resolve its standard Parent property. Native 1C distinguishes Parent writes by the catalog hierarchy setting; assuming that every catalog accepts the same writes would be incorrect.

## What Changes
Add Parent and its Russian alias to new catalog objects and their existing finite receiver context. Retain the acquired hierarchy setting in ORM metadata. Implement measured nil reference defaults and assignment behavior, preserving object selection timing and explicit failure when the required hierarchy setting is unknown. Add the reference IsEmpty method used to observe that value.

## Impact
bsl-orm catalog object and reference method/property tables, bsl-config-db metadata projection, native fixture replay, generated API reference. No database writes or new dependencies.
