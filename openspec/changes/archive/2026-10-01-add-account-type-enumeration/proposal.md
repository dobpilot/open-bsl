# Add the reached AccountType system enumeration

## Why
Original UNF ChartOfAccounts.Управленческий.ManagerModule uses ВидСчета.Активный, Пассивный and АктивноПассивный. The system enumeration is absent and prevents unchanged source from linking.

## What Changes
Add the complete vendor-defined ВидСчета / AccountType enumeration through existing enum tables, preserving resolver priorities. Measure value and type presentations on native 1C before implementation.

## Impact
Runtime enumeration tables, focused CLI/oracle tests and generated API reference. No configuration-specific enumeration replacement or new dependencies.
