## ADDED Requirements

### Requirement: Execute accepts a bare expression
The `Выполнить` and `Execute` statements SHALL accept their source-string expression either directly or enclosed in parentheses. Both forms SHALL evaluate the expression and execute the resulting dynamic source in the same caller scope.

#### Scenario: Bare concatenated expression
- **WHEN** an Execute statement is followed directly by a concatenated string expression
- **THEN** the resulting source is executed in the caller scope

#### Scenario: Parenthesized expression remains supported
- **WHEN** an Execute statement encloses its expression in parentheses
- **THEN** it retains the existing dynamic execution behavior
