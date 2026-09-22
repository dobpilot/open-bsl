# Design

The Syntax Assistant 8.3.27.2342 documents the parenthesized spelling `Выполнить(<Строка>)`. An authorized scan of the platform-accepted `unf` configuration found the keyword followed directly by a concatenated expression. A focused measurement on 1C:Enterprise 8.3.27.2342 confirmed that `Выполнить "Число40 = 42";` executes and changes the caller's variable to 42.

The parser will parse one ordinary expression immediately after `Выполнить`. Parentheses require no separate branch because a parenthesized expression is already part of the expression grammar. The statement boundary remains the existing semicolon or enclosing block boundary. Downstream stages continue to receive the same `StmtKind::Execute` node.

After implementation, the authorized `unf` compile gate passed this statement and reached the next independent parser barrier: a string literal where `)` was expected. The same three previously known unreadable `.7` resources were skipped.
