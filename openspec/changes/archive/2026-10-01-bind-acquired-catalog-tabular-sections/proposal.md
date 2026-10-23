# Bind acquired catalog tabular section names

## Why
Original UNF Catalog.ГруппыПользователей.ObjectModule uses the bare receiver property Состав. Native configuration XML identifies it as an owned tabular section, distinct from its nested Пользователь field. Current receiver contexts omit tabular sections and reject the unchanged module.

## What Changes
Acquire authoritative ordered owner section declarations through open-sdbl, retain their names in ORM metadata and include them in existing finite object-module receiver contexts. Until typed section values are implemented, actual access fails explicitly; declarations do not fabricate empty collections.

## Impact
Affects bsl-config-db, bsl-orm and bsl-session. Existing facade resolver priority and core dependencies remain unchanged. Real row operations remain part of the configuration runtime implementation, not satisfied by this declaration change.
