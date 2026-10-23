# Expose the acquired empty configuration default interface

## Why
Original BUH session initialization reads Metadata.DefaultInterface while creating a user description. Its native configuration has no default interface. The ORM currently rejects the property itself.

## What Changes
Declare read-only ОсновнойИнтерфейс / DefaultInterface on the existing metadata root. Return Undefined only for an explicitly acquired nil default-interface UUID. Preserve explicit failures for unacquired data and currently unsupported non-nil interfaces. Add one immutable catalog builder for the acquired UUID and project the measured native root field through existing SDK Values.

## Impact
Public ORM Rust and BSL API and the generated reference change. Existing resolver, VM, session dispatch, SDK API and dependencies remain unchanged. This does not implement ordinary Interface metadata objects.
