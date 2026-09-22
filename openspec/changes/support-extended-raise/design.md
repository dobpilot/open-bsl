# Design

The source of the signature is the installed 1C:Enterprise Syntax Assistant 8.3.27.2342, document `platform-syntax-help:platform-guides:struct_Raise`. A configuration module from the `unf` test information base independently demonstrates that the parenthesized form with omitted middle positions occurs in platform-accepted production source.

The existing `Raise` AST and bytecode instruction continue to represent `ВызватьИсключение Выражение` and the bare rethrow form. A separate structured AST statement and bytecode instruction represent the parenthesized form. This keeps old bytecode behavior unchanged and makes the five positional values explicit. The compiler materializes omitted positions as `Неопределено` in five consecutive registers; the VM applies the documented defaults and validates the supplied types. The textual bytecode format version advances because the instruction set changes.

The structured form creates the same immutable `ИнформацияОбОшибке` object that an exception handler observes through `ИнформацияОбОшибке()`. It stores description, category, code, additional information, and an optional cause. `ЯвляетсяОшибкойКатегории` checks the stored category and, when requested, the cause chain; `ВсеОшибки` remains true. The existing bridge for VM errors and background jobs continues to create the same object with the fields available to those sources.

The exact runtime details will be checked on 1C 8.3.27.2342. Until that round trip is ingested, the implementation site carries `НЕ ИЗМЕРЕНО(ERROR.EXTENDED_RAISE)` together with the registry and measurement-script entries required by the repository.
