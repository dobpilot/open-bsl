# Design

The installed 1C:Enterprise Syntax Assistant 8.3.27.2342 documents the rule in `platform-syntax-help:platform-guides:def_String`: only whitespace, line breaks, and comment lines may occur between the fragments of a multiline string, and a comment is allowed when its line starts with `//`. The general source-symbol reference `platform-syntax-help:platform-guides:source_specsymbols.html` defines `//` as a comment through the end of the current line.

A focused measurement on 1C:Enterprise 8.3.27.2342 used an active first fragment, one intervening comment line, and an active `|` fragment. The resulting value contained the two active fragments separated by exactly one line-feed character. The comment text and its physical line break did not contribute to the value. A second case with horizontal whitespace before `//` produced the same value.

The lexer will therefore defer adding the logical line break until it finds the next active continuation fragment. After the end of a fragment it will skip horizontal whitespace and any complete line whose first non-whitespace characters are `//`; it will then require `|` on the next active line and append one `\n` to the value. This keeps comment handling inside the existing string scanner and does not add a second source preprocessor.

The implementation is deliberately limited to comments between multiline string fragments. It does not change ordinary comment tokens or the existing treatment of whitespace before `|`.

After the change, the authorized `unf` compile gate passed the former `ExpectedContinuationBar` location and reached the next independent parser barrier: a `+` token where an expression begins. The same three previously known unreadable `.7` resources were skipped; this lexer change does not alter their handling.
