## ADDED Requirements

### Requirement: Filling-check metadata preserves acquisition completeness
The ORM metadata API SHALL retain acquired DontCheck and ShowError settings for fields and the complete ordered standard-field settings of a catalog. Unknown settings SHALL remain distinct from known absence of checks. The database adapter SHALL project SDK settings without inferring them from field lengths or values.

#### Scenario: Standard settings are unknown
- **GIVEN** a catalog descriptor with an unknown standard filling-check table
- **WHEN** its ORM metadata is built
- **THEN** the table remains unknown rather than becoming an empty table

#### Scenario: Field setting is acquired
- **GIVEN** an acquired field descriptor with ShowError
- **WHEN** the database adapter projects the field
- **THEN** ORM metadata retains ShowError

### Requirement: Catalog primitive attributes participate in filling checks
Catalog object instances SHALL retain acquired own string, number, Boolean and date attribute descriptions separately from common attributes, with values stored per object instance. The database adapter SHALL preserve the complete acquired own attribute list and filling-check modes. Unknown types or incomplete acquisition SHALL produce explicit errors on the affected operation.

#### Scenario: Native own string attribute
- **GIVEN** the preserved native catalog fixture with ПроверяемаяСтрока configured as a variable string of length 30 and ShowError
- **WHEN** Description is filled and the own string is empty
- **THEN** CheckFilling returns FALSE
- **AND** filling the own string with test makes it return TRUE

#### Scenario: Own attribute message uses the configuration current-language synonym
- **GIVEN** the native own string fixture with English synonym Own attribute title
- **WHEN** its empty value is checked in an English session
- **THEN** the message text is "Own attribute title" is a required field.
- **AND** the message field remains ПроверяемаяСтрока
- **AND** the ORM retains acquired localized synonyms separately from the canonical attribute name

#### Scenario: Native scalar checks
- **GIVEN** the preserved native scalar catalog fixture with Description and the own string filled
- **WHEN** number and date remain empty
- **THEN** validation returns FALSE and emits messages for the number followed by the date
- **AND** after populating number and date validation returns TRUE with either Boolean value

#### Scenario: Applied common string checks follow own attributes
- **GIVEN** the native catalog fixture with own ПроверяемаяСтрока and applied common НаименованиеЯзык1 configured as ShowError
- **WHEN** Description is filled and both strings are empty
- **THEN** the handler receives Description, ПроверяемаяСтрока and НаименованиеЯзык1 in that order
- **AND** validation emits messages for the own field followed by the common field
- **AND** filling only the own field still returns FALSE and filling both returns TRUE

#### Scenario: Message and attribute language differ
- **GIVEN** the native fixture whose current configuration language is en and system language is ru
- **WHEN** the empty own attribute is checked
- **THEN** the message is Поле "Own attribute title" не заполнено
- **AND** the attribute caption uses the configuration language independently of the system-language message template

### Requirement: Catalog objects validate filling through configuration rules
Catalog objects SHALL expose zero-argument ПроверитьЗаполнение and CheckFilling methods returning a Boolean result derived from acquired configuration filling-check rules and the object instance's filling-check handler. Missing rules SHALL produce an explicit error rather than a fabricated successful result.

#### Scenario: Empty and populated native probe objects
- **GIVEN** the acquired configuration represented by the preserved native catalog filling-check fixture
- **WHEN** the host checks an empty object and then the same object with Code and Description populated
- **THEN** the results match the native FALSE and TRUE receipts respectively

#### Scenario: Check during object-module initialization
- **WHEN** the preserved native object module calls ПроверитьЗаполнение in its initialization body
- **THEN** the operation raises an error as in the native receipt

#### Scenario: Unknown validation settings
- **GIVEN** a catalog whose filling-check settings could not be acquired
- **WHEN** CheckFilling executes
- **THEN** an explicit unsupported acquisition error is returned

#### Scenario: Alias and arity
- **WHEN** the populated native probe object is checked through CheckFilling without arguments
- **THEN** it returns TRUE
- **AND** passing an argument is rejected

#### Scenario: Handler removes checks or refuses validation
- **GIVEN** the native handler fixture with an empty catalog object
- **WHEN** the handler clears the checked-field array
- **THEN** validation returns TRUE
- **AND** clearing the array while setting refusal returns FALSE

#### Scenario: Native standard field paths
- **GIVEN** the native handler fixture with empty Code and Description
- **WHEN** the handler replaces the checked-field array with the single path Code or Description
- **THEN** validation returns FALSE
- **AND** the measured paths Код, Наименование and НесуществующийРеквизитПробы return TRUE
