# Tasks

- [x] 1. Add failing lexer and runtime tests for a twelve-digit literal and retained rejection of unsupported lengths.
- [x] 2. Accept the twelve-digit form in the lexer and parse omitted seconds as zero in the runtime.
- [x] 3. Measure the resulting date components on 1C 8.3.27.2342 in a session whose anchors all match.
- [x] 4. Measure and implement the documented rule that date literals ignore non-digit separators.
- [x] 5. Resume the authorized `unf` compile gate and record the next structural barrier without exposing private source.
- [ ] 6. Run formatting, focused tests, Clippy, workspace tests, and rustdoc.
- [ ] 7. Validate with `openspec validate support-date-literal-without-seconds --strict`; archive only after all checks pass.
