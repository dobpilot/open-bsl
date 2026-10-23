# Support reached value list value unloading

## Why
After measured presentation sorting, original demo initialization reaches ValueList.UnloadValues, which is absent.

## What Changes
Add the Russian method name and English alias to the existing value list component. Return an independent array containing its item values in their current order.

## Impact
Only the existing component method table and implementation change. No dependencies, new interpreter or changes to other list methods are required.
