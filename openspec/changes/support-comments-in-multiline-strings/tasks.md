# Tasks

- [x] 1. Add a failing lexer test for a comment line between active multiline string fragments.
- [x] 2. Teach the string lexer to skip intervening full-line comments while preserving one logical line break.
- [x] 3. Resume the authorized `unf` compile gate and record the next structural barrier without exposing private source.
- [x] 4. Run formatting, focused tests, Clippy, workspace tests, and rustdoc.
- [ ] 5. Validate with `openspec validate support-comments-in-multiline-strings --strict`; archive only after all checks pass.
