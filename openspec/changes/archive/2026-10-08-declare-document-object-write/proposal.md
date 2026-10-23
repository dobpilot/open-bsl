# Declare document object Write

## Why
Original UNF Document.Событие.ObjectModule contains bare Write calls in procedures. The existing receiver context has no declaration, so compilation fails before those procedures are invoked.

## What Changes
Declare DocumentObject.Write / Записать with zero through two arguments. Preserve existing exported object-module methods. Execution remains an explicit unsupported-persistence error until a separate adapter implementation connects writes.

## Impact
Affects the ORM document receiver, generated BSL reference and Session tests. No new dependencies or resolver priority changes. This intermediate declaration does not complete original session startup or persistence.
