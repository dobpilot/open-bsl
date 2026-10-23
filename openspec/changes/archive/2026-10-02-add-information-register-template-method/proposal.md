# Add the information register manager template method

## Why

Original UNF manager source for СеансовыеДанныеGoogle calls ПолучитьМакет.
The finite receiver table omits this documented method, so dormant original
procedures cannot link.

## What Changes

- Declare ПолучитьМакет / GetTemplate on information register managers with
  one required argument, using the existing receiver context.
- Preserve existing manager exports and variable precedence.
- Retain explicit failure until real acquired template resources are available.

## Impact

The ORM receiver table, focused Session tests and generated reference are
affected. No resolver, VM, SQL access or dependency boundary changes.
