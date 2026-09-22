# Design

The Syntax Assistant 8.3.27.2342 states that multiline string constants have two source forms and permits only whitespace, line breaks, and comment lines between their fragments. The installed help loses the list formatting, so focused platform measurements established the missing value rule. Adjacent quoted fragments separated on the same line, by a physical line break, or by a full comment line all produced one LF between the fragment values.

The lexer already removes trivia and emits one `String` token per quoted fragment. The parser will therefore consume a maximal run of adjacent `String` tokens in `parse_primary`, append one `\n` before each later fragment, and produce the existing single `Expr::Str`. This keeps the source distinction out of semantic analysis and bytecode because it has no runtime effect after parsing.

After implementation, the authorized `unf` graph passed lexing and parsing in full. Engine construction then reached a duplicate global export from two common modules. The next investigation therefore moves from syntax to the existing session-factory mapping of module execution attributes. The same three previously known unreadable `.7` resources were skipped.
