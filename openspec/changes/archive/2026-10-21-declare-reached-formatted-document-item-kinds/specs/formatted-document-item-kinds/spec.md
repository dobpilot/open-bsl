## ADDED Requirements

### Requirement: Reached formatted document item kinds
The runtime SHALL expose ПереводСтроки/LineFeed and Картинка/Picture in the existing ТипЭлементаФорматированногоДокумента/FormattedDocumentItemType system enum. Their presentations SHALL be LineFeed and Picture, respectively. Existing Текст/Text behavior and presentation SHALL remain unchanged. All three members SHALL have the existing FormattedDocumentItemType identity and English type presentation. Type lookup SHALL accept both measured language spellings. LineBreak and Image SHALL remain invalid aliases. This declaration SHALL NOT implement formatted-document operations.

#### Scenario: Reached original member references
- **WHEN** dormant BUH code refers to ПереводСтроки or Картинка
- **THEN** the existing resolver accepts the members
- **AND** their English aliases identify the same respective values

#### Scenario: Preserve type and text behavior
- **WHEN** code reads any of the three measured members
- **THEN** its type and presentation match the native control
- **AND** both Russian and English type lookup names identify the same type

#### Scenario: No guessed aliases
- **WHEN** code requests LineBreak or Image
- **THEN** the member remains undefined
