# Add the reached empty document reference

## Why
Original UNF Document.ЗаказПокупателя.ManagerModule uses bare ПустаяСсылка(). Document managers lack this method and the runtime currently declares owned reference types only for catalogs.

## What Changes
Measure native empty document references, then add Document manager EmptyRef through existing receiver method contexts and immutable owned value descriptions. Reuse the existing reference identity, UUID and transfer machinery while retaining catalog-only public helper contracts. Do not fabricate a catalog reference or Undefined for a document.

## Impact
bsl-orm reference type acquisition, document manager methods, native probes and focused Session tests. No core or VM dependency changes, new resolver path, or database record writes.
