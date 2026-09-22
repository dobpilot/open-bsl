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
