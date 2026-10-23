# Add catalog manager selection

## Why
Original UNF Catalog.Валюты.ManagerModule calls bare Выбрать().Следующий(). The existing receiver lacks catalog selection.

## What Changes
Add catalog-manager Select and a configuration-owned selection value using the principal-bound data adapter. Preserve native iteration and field access behavior, aliases and receiver priority. No fabricated empty selection or successful no-op database operations.
