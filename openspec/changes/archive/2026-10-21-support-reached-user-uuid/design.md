# Design

Native own PostgreSQL control on 8.3.27.2342 confirms UUID type, UUID English alias, identity equality with FindByName and refused assignment. The native UUID matches the first GUID of the own UserData record, already decoded by the SDK into Principal.id. Evidence is retained under docs/research/fixtures/user-infobase-uuid-native-1015, including the exact same-version vendor help locator. Original demo receiver and function are recorded in demo-db-settings-1015-startup.log.

Read the current user object's frozen Principal and parse its identifier with the existing UUID codec. Invalid host identifiers fail explicitly rather than returning a generated/nil identifier. Return the existing BslObject::Uuid representation. The descriptor has no setter. No database read, mutable state or special VM support is added. New-user behavior is not implemented.

## Verification

Four focused UUID tests pass. The generated API reference is current. A frozen full workspace run passes 3627 tests, with zero failures and one ignored dump; workspace Clippy, warning-free rustdoc, formatting and strict OpenSpec pass. Original UNF replay passes the property and reaches a new unsupported UUID query parameter. Original startup as a whole is not claimed successful.
