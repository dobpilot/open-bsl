# Design

The installed 1C:Enterprise Syntax Assistant 8.3.27.2342 documents unary `+` and `-` in `platform-syntax-help:platform-guides:OperationUnar`, limits both operations to numbers, and assigns them the same precedence. A focused platform measurement showed that `+7` produces `7` and that `+"text"` raises a catchable runtime error. The authorized `unf` source scan independently found unary `+` in the argument position where compilation currently stops.

`UnaryOp` will gain `Pos`, and the parser will collect both unary signs before folding them from right to left. This preserves the existing right-associative treatment of unary chains and evaluates the operand once.

The compiler will lower `Pos` to two existing `Neg` instructions on the same temporary register. The first instruction performs the existing numeric type check and negates the value; the second restores the original number. This avoids a new bytecode opcode and format-version change for an operation whose only runtime effect beyond validation is identity. Decimal negation is exact in `BslNumber`, so the pair does not round or otherwise alter the value.

After the change, the authorized `unf` compile gate passed the unary-plus location and reached the next independent lexer barrier: an empty physical line between active multiline-string fragments, followed by a `|` continuation line. The same three previously known unreadable `.7` resources were skipped; this syntax change does not alter their handling.
