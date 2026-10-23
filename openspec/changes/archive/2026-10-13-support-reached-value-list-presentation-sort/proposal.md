# Support reached value list presentation sorting

## Why
Original demo session initialization reaches ValueList.SortByPresentation, which is currently absent. Native controls establish direction handling and presentation ordering.

## What Changes
Add the Russian method name and English alias to the existing value list component. Preserve item identity and use measured presentation ordering. Retain native controls and regenerate the public API reference.

## Impact
The existing runtime component gains one method. No dependencies, resolver changes, alternate interpreter or unrelated value sorting changes are introduced.
