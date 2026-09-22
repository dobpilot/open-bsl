## ADDED Requirements

### Requirement: Adjacent quoted fragments form one multiline string
The language SHALL combine adjacent quoted string fragments separated only by trivia into one string value. It SHALL insert exactly one LF between each pair of fragments.

#### Scenario: Same-line fragments
- **WHEN** two quoted fragments are separated by horizontal whitespace
- **THEN** their values are joined with one LF

#### Scenario: Fragments on separate lines
- **WHEN** two quoted fragments are separated by a physical line break
- **THEN** their values are joined with one LF

#### Scenario: Comment between fragments
- **WHEN** two quoted fragments are separated by a full comment line
- **THEN** their values are joined with one LF and the comment contributes nothing
