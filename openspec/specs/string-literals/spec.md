# string-literals Specification

## Purpose
Define the source forms and resulting values of BSL string literals.

## Requirements

### Requirement: Multiline strings allow intervening comment lines
The language SHALL allow a full-line `//` comment between active fragments of a multiline string literal. It SHALL exclude the comment text and the comment line's physical line break from the resulting value, and SHALL preserve exactly one logical line break between the surrounding active fragments.

#### Scenario: Comment between active fragments
- **WHEN** a multiline string has an initial fragment, a full-line `//` comment, and a following `|` fragment
- **THEN** lexing succeeds
- **AND** the string value consists of the two active fragments separated by one line-feed character

#### Scenario: Active fragment remains mandatory
- **WHEN** a multiline string has a full-line comment after an active fragment but no following `|` fragment
- **THEN** lexing fails with the existing missing-continuation diagnostic

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
