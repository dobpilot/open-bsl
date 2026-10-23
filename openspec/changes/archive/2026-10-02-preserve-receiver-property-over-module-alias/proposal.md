# Preserve receiver properties over module aliases

## Why
Original demo catalog object code calls ФизическиеЛица.Сортировать on an owned receiver property, but imported module alias selection ignores the receiver property context.

## What Changes
Prevent alias selection when the base name is a declared receiver property. Preserve existing variable priority and modules without receiver context.

## Impact
Affects sema alias lookup and focused facade/Session regressions. No new API or dependency and no changes to library global-name ordering.
