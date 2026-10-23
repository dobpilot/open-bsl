# Why
Original BUH compilation reaches a bare ПолучитьПоследнее call in the НастройкиУчетаУСН information-register manager module. The runtime receiver table declares related register methods but omits GetLast.

# What Changes
Declare ПолучитьПоследнее/GetLast with zero through two arguments using the existing information-register manager receiver mechanism. Unsupported periodic-register execution remains an explicit failure; no records or empty structures are fabricated.

# Impact
ORM manager method tables, focused facade/session tests and generated API reference. No resolver priorities, VM operations, SDK dependencies or database writes change.
