## ADDED Requirements

### Requirement: Multiline strings allow intervening empty lines
The language SHALL allow empty physical lines between active fragments of a multiline string. It SHALL exclude those empty lines from the value and preserve exactly one logical line break between the surrounding active fragments.

#### Scenario: Empty line between fragments
- **WHEN** an empty physical line occurs before the next `|` fragment
- **THEN** lexing succeeds and the active fragments are separated by one LF

### Requirement: Multiline string line endings are normalized
The language SHALL treat LF and CRLF source line endings as one physical line ending and SHALL represent the logical line break between active fragments as LF.

#### Scenario: CRLF source
- **WHEN** a multiline string and its intervening empty line use CRLF
- **THEN** the resulting value contains one LF between active fragments and no CR characters
