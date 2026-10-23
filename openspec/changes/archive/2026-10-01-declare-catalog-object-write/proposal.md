# Declare catalog object Write

## Why
UNF Catalog.МобильныеПриложенияРаспознаванияДокументов.ObjectModule contains bare Записать() in the exported Отключить procedure. Compilation fails before it is known whether startup invokes this procedure.

## What Changes
Declare the platform zero-argument Write method on the existing object receiver so original source can compile. Invocation must report explicitly that persistence is not connected; it must not report success, modify a database, or manufacture a persisted reference. Actual persistence remains an open runtime task.
