# Design

The source of the signature is the installed 1C:Enterprise Syntax Assistant 8.3.27.2342, document `platform-syntax-help:platform-guides:root_New`. Its second syntax variant is `Новый(<Тип>[, <ПараметрыКонструктора>])`: the first argument is a type name or a `Тип` value, the second is an array of constructor arguments, and a second argument of another type is ignored. A module from the `unf` test information base independently demonstrates that the parenthesized form occurs in platform-accepted production source.

The parser gives the functional form its own AST node when `Новый` is immediately followed by `(`. The existing `Новый ИмяТипа(...)` node and its compile-time resolution remain unchanged. The semantic form contains the type expression and the optional parameter expression, so both preserve normal expression evaluation order.

A separate bytecode instruction carries registers for the runtime type and optional parameter value. The textual bytecode format version advances because the instruction set changes. The VM resolves a string through the existing `RuntimeShapes::resolve_type`; a `Тип` value is used directly. The instruction expands the optional value only when it is an array. It then dispatches native types to the same runtime value constructors already used by the static opcodes, and component object types to the `ConstructorDescriptor` already registered for their `TypeDescriptor`. This avoids a second component constructor registry and keeps host capabilities in the ordinary `CallContext`.

The dynamic result is not classified as a statically known core receiver: the selected type is known only during execution, so subsequent member access follows the existing open-object path.

The exact platform diagnostics and boundary behavior will be checked on 1C 8.3.27.2342. Until that round trip is ingested, the implementation site carries `НЕ ИЗМЕРЕНО(CONSTRUCTOR.FUNCTIONAL_NEW)` together with the registry and measurement-script entries required by the repository.
