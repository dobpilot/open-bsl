# Bind acquired object attributes

## Why
Original UNF Catalog.ExternalUsers.ObjectModule reads its actual metadata attribute ObjectAuthorization. The current receiver property table contains standard catalog properties only, so original source fails semantic resolution.

## What Changes
Retain a finite acquired attribute-name collection per catalog in ORM metadata and expose owned property-name contexts through the open-bsl facade. Reuse normal receiver property instructions for static and dynamic module code. Until a typed value implementation is provided for an acquired attribute, property access must fail explicitly rather than fabricate an Undefined value.

## Impact
open-bsl property context storage, sema and dynamic compilation inputs, configuration worker recipes, bsl-orm metadata and catalog object dispatch, bsl-config-db projection. No new external/core dependency, SQL parser or source rewriting. Full typed attribute values remain required when reached on the executed startup path.
