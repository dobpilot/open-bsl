# Add catalog folder creation

## Why

Original demo and unf both reach unresolved СоздатьГруппу in a catalog manager module. The SDK now acquires hierarchy kind, and four native probes establish behavior independently of the hierarchy flag.

## What Changes

Retain acquired hierarchy kind and expose the zero-argument СоздатьГруппу/CreateFolder manager method. Reuse existing new-object creation and initialization, setting its folder flag after the module body where native behavior allows it. Preserve explicit failures for unknown settings and item-only catalogs.
