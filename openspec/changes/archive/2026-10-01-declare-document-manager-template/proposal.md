# Declare the reached document manager template method

## Why
After resolving AccountType, unchanged UNF Document.ЗаказПокупателя.ManagerModule fails at ПолучитьМакет. Its template retrieval calls occur in the original manager source and the document receiver context does not declare the platform method.

## What Changes
Add the verified one-argument ПолучитьМакет / GetTemplate declaration for document managers through existing method tables. Preserve the existing data processor behavior. Actual retrieval remains explicitly unavailable until real template resources are supported.

## Impact
bsl-orm document manager methods and focused Session tests. No resolver, core dependency or source rewriting changes.
