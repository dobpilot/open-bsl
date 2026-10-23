# Stage the reference of a new catalog object

## Why

Original UNF Catalog.Организации.ObjectModule calls GetNewObjectRef and SetNewObjectRef while preparing a new item. The current finite catalog-object method table lacks both methods, stopping session compilation.

## What Changes

Measure native behavior, then add the staged-reference state to the existing per-object receiver. Reuse owned immutable catalog references; do not implement database writes or another object model.
