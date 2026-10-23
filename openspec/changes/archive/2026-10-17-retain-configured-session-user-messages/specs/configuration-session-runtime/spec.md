## ADDED Requirements

### Requirement: Configured Session pending user messages are readable
A configured Session SHALL retain user-message DTOs awaiting consumption and
expose them through GetUserMessages without replacing them with an empty
result. The returned fixed array SHALL contain user-message objects preserving
the measured text and attribute field. Reading with removal SHALL return the
pending messages and remove those entries. Each Session SHALL own its pending
messages independently. Existing hosts without pending-message support SHALL
retain their existing empty-result behavior.

#### Scenario: Read a failed filling-check message
- **WHEN** the original measured CheckFilling control rejects its required string and reads pending messages with removal
- **THEN** all four native message-control lines match, including one message with its measured text and field

#### Scenario: Preserve the standalone host
- **WHEN** a standalone State uses the existing immediate-output host without pending-message support
- **THEN** message output and empty GetUserMessages behavior remain unchanged

#### Scenario: Keep Session collections separate
- **WHEN** another configured Session opens after the first Session has pending messages
- **THEN** it cannot read the first Session's pending messages
- **AND** a background Session does not inherit its parent's pending collection

#### Scenario: Preserve background-job history
- **WHEN** a Session consumes its pending user messages
- **THEN** it does not remove or rewrite the background-job registry's historical message collection

#### Scenario: Preserve the measured message identity
- **WHEN** the source message is changed after sending
- **THEN** the queued message retains its sent text and field
- **AND** changing the object returned by a non-removing read is visible in later reads and the removing read
