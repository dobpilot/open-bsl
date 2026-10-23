## ADDED Requirements

### Requirement: Acquired document attribute metadata view
Document metadata SHALL expose Реквизиты and Attributes as aliases of an immutable view of acquired document attributes. The view SHALL preserve acquired order and identity, support property-name lookup, string-name and numeric indexing, Количество/Count, Получить/Get and Найти/Find. Name lookup SHALL ignore case; missing Find SHALL return Undefined. Unknown acquired collection data SHALL fail explicitly; a known empty collection SHALL have zero members.

#### Scenario: Native ordered and named access
- **WHEN** the native fixture's document has СсылкаПробы followed by СоставнойПробы
- **THEN** Count SHALL return two, numeric index zero SHALL return СсылкаПробы and Get one SHALL return СоставнойПробы
- **AND** case-insensitive Find and name indexing SHALL return the same attribute identity as property lookup

#### Scenario: Unknown versus empty
- **WHEN** the document has no acquired attribute collection
- **THEN** Attributes SHALL fail explicitly
- **WHEN** its acquired collection is empty
- **THEN** Count SHALL return zero without manufacturing members

### Requirement: Read-only acquired attribute filling properties
Attribute metadata SHALL expose Имя/Name, ЗаполнятьИзДанныхЗаполнения/FillFromFillingValue and ЗначениеЗаполнения/FillValue as read-only acquired properties. FillValue SHALL restore the acquired single-root graph using the invoking State's materializers. Absent data and malformed root counts SHALL fail explicitly rather than substitute Undefined or an empty reference.

#### Scenario: Missing enumeration filling reference
- **WHEN** the acquired FillValue contains a nonempty enumeration UUID absent from its value list
- **THEN** reading FillValue SHALL preserve its concrete type, identity and filled state
- **AND** the complete native missing-reference probe SHALL reproduce metadata read, creation and filling observations for each measured UI language

#### Scenario: Unknown and malformed filling data
- **WHEN** the filling flag or value was not acquired, or the value graph has a root count other than one
- **THEN** reading the corresponding property SHALL fail explicitly

#### Scenario: Read-only metadata
- **WHEN** BSL attempts to set an attribute metadata property
- **THEN** the operation SHALL fail and acquired metadata SHALL remain unchanged

### Requirement: Measured generic metadata value type
Attribute metadata and its document metadata owner SHALL share the existing generic metadata value type, with the measured type presentation Metadata object. Existing metadata value lookup names SHALL remain available; the runtime SHALL NOT invent a separate ОбъектМетаданныхРеквизит type.

#### Scenario: Generic metadata type identity
- **WHEN** code compares the type of an acquired attribute metadata value with its document metadata owner
- **THEN** the types SHALL be equal and String of the type SHALL return the measured Metadata object presentation
- **AND** lookup of the invented ОбъектМетаданныхРеквизит type SHALL retain the existing unknown-type failure
