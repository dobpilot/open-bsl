# Add catalog predefined flag

## Why
Original demo ГруппировкиЗадач.ObjectModule accesses Предопределенный, absent from the receiver property table.

## What Changes
Expose read-only Предопределенный / Predefined on the supported new catalog object, derived from its existing predefined name.

## Impact
ORM property table, generated API reference and native replay tests. No new dependency, stored flag or database persistence.
