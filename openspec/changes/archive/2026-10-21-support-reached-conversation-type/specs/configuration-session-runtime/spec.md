## ADDED Requirements

### Requirement: Conversation identifier type lookup without a service
The session library SHALL resolve Type("ИдентификаторОбсужденияСистемыВзаимодействия") and Type("CollaborationSystemConversationID") to the same type, with the measured presentation CollaborationSystemConversationID, without requiring a collaboration host service.

#### Scenario: Guarded source type comparison
- **WHEN** configuration code compares either name with the type of an ordinary value
- **THEN** type lookup SHALL succeed without creating an identifier value

#### Scenario: Preserved unsupported construction
- **WHEN** code constructs an identifier without a supported host implementation
- **THEN** the existing catchable explicit unsupported error SHALL remain
