# characteristic-object-instances Specification

## Purpose
Define acquired characteristic metadata and isolated configuration object instances through the existing ORM and module-instance bridge.

## Requirements

### Requirement: Acquired characteristic metadata
The ORM SHALL consume host-supplied characteristic settings and configured value types without decoding Config or substituting catalog defaults.

#### Scenario: Unknown settings
- **GIVEN** a characteristic manager whose creation settings are unknown
- **WHEN** an element is requested
- **THEN** creation fails explicitly rather than inventing settings

### Requirement: Characteristic creation
A characteristic manager SHALL create a separate in-memory object of its own configured characteristic type without writing it to the database.

#### Scenario: Native default state
- **GIVEN** the independently measured four-type characteristic configuration
- **WHEN** an element is created through the facade
- **THEN** its type, empty code/description/reference, false folder flag, true new flag and four configured value types match the native receipt

### Requirement: Characteristic module initialization
Creation SHALL initialize the existing object-module instance before exposing the selected new element.

#### Scenario: Measured module body
- **GIVEN** the measured object module that reads Code, Description and ValueType and tries Description assignment and IsNew
- **WHEN** its manager creates an element
- **THEN** body reads and errors match the native receipt
- **AND** configured value types and the new-element state become available after the body

#### Scenario: Independent instances
- **GIVEN** two elements created from the same manager
- **WHEN** one element changes its in-memory state
- **THEN** the other element and another session retain independent instance state

### Requirement: Existing manager context
Bare supported characteristic methods in their own manager module SHALL use the existing configuration method context.

#### Scenario: Bare creation
- **GIVEN** a characteristic manager module with a bare CreateItem call
- **WHEN** the factory compiles the original module
- **THEN** the method resolves through its own manager context with existing local and module name priorities preserved

### Requirement: Measured characteristic property assignment
Characteristic standard property assignment SHALL follow the measured native qualification and selection restrictions.

#### Scenario: String lengths and configured value types
- **GIVEN** the measured characteristic with code length 9, description length 25 and four configured value types
- **WHEN** longer strings are assigned to Code and Description
- **THEN** the values are truncated to their configured lengths
- **WHEN** a string-only TypeDescription is assigned to ValueType
- **THEN** ValueType contains that one type
- **WHEN** Undefined is subsequently assigned to ValueType
- **THEN** ValueType is a TypeDescription containing the original four configured types

#### Scenario: Read-only folder flag
- **GIVEN** a newly created characteristic element
- **WHEN** the application assigns its folder flag
- **THEN** assignment fails as in the native receipt

### Requirement: TypeDescription type enumeration
TypeDescription.Types SHALL return a fresh mutable array of its represented type values.

#### Scenario: Independent type array
- **GIVEN** the measured string-and-number TypeDescription
- **WHEN** its Types array is cleared
- **THEN** the next Types call still returns the original two types

### Requirement: Characteristic exchange parameters
A characteristic object SHALL expose its own exchange parameters through the existing ORM exchange-settings object.

#### Scenario: Independent loading flags
- **GIVEN** two new characteristic elements
- **WHEN** the first element's exchange Loading flag is set to true
- **THEN** its flag is true and the second element's flag remains false
- **AND** recipient AutoFill initially is true

### Requirement: Acquired characteristic own attributes
The ORM SHALL accept a complete host-supplied characteristic own-attribute list, distinguish unknown from known empty lists, and bind acquired names to the existing object-module receiver context.

#### Scenario: Measured string attribute
- **GIVEN** the measured characteristic with its twelve-character string attribute
- **WHEN** an element is created
- **THEN** the attribute reads as an empty string during the module body and rejects assignment until selection
- **WHEN** a longer string is assigned after creation
- **THEN** its value is truncated to twelve characters and a second element retains an empty string

#### Scenario: Unknown attribute metadata
- **GIVEN** a characteristic whose own-attribute metadata is unknown
- **WHEN** an unknown nonstandard property is accessed
- **THEN** it fails explicitly without manufacturing a module variable or guessing its type

### Requirement: TypeDescription membership
TypeDescription.ContainsType SHALL test whether a supplied type value occurs in the represented type list.

#### Scenario: Measured membership and alias
- **GIVEN** the measured string-only TypeDescription
- **WHEN** its membership method receives String and Number type values
- **THEN** it returns true and false respectively
- **AND** the Russian and English method names return the same result

### Requirement: Preserve configured value type qualifiers
The characteristic value type description SHALL retain acquired qualifiers and apply the independently measured conversion behavior without changing existing standalone type description behavior.

#### Scenario: Apply configured string and number bounds
- **GIVEN** a characteristic configured with Boolean, variable String(100), DateTime and Number(15,2) of either sign
- **WHEN** its value type description adjusts values
- **THEN** a 120-character string becomes 100 characters, 1.234 becomes 1.23, -1.235 becomes -1.24, and 12345678901234.56 becomes 9999999999999.99
- **AND** Boolean and DateTime inputs retain their values

#### Scenario: Default of the measured compound description
- **GIVEN** the same configured compound value type description
- **WHEN** conversion receives Undefined, Null or no argument
- **THEN** the result is Undefined

#### Scenario: Preserve qualifiers through internal text
- **GIVEN** the measured configured compound value type description
- **WHEN** it is converted to the platform internal text and read back
- **THEN** String(100, variable) and Number(15,2, either sign) qualifiers remain present
- **AND** the reconstructed description retains the measured truncation and rounding

#### Scenario: Use the configured description in a typed column
- **GIVEN** the measured configured compound value type description
- **WHEN** it is supplied to a value table column
- **THEN** assignment of a 120-character string retains 100 characters
- **AND** assignment of 12345678901234.56 retains 9999999999999.99

### Requirement: SDK characteristic projection
The database configuration adapter SHALL project acquired characteristic settings, qualified value types and ordered own attributes through the existing ORM catalog API. Unknown and known empty data SHALL remain distinct. Unresolved configured value types SHALL fail explicitly.

#### Scenario: Native SDK resource
- **GIVEN** the independently measured characteristic resource acquired by SDK 0.9.7
- **WHEN** the database adapter projects its metadata
- **THEN** the configured lengths, qualified value types and own attributes match the native configuration
- **AND** characteristic reference types use the existing configured reference type declarations

#### Scenario: Unknown and empty metadata
- **GIVEN** unknown characteristic settings or an unknown own-attribute list
- **WHEN** projection completes
- **THEN** those values remain unknown
- **AND** a verified empty list remains known empty

### Requirement: Characteristic additional properties
Each characteristic object SHALL expose its own mutable AdditionalProperties structure through a read-only property. The structure SHALL be available during object-module initialization and remain the same structure after selection. Russian and English property names SHALL resolve to it.

#### Scenario: Native initialization and isolation
- **GIVEN** the independently measured characteristic object module that records the initial structure count and inserts its body value
- **WHEN** two elements are created
- **THEN** each initially empty structure retains its own body value after creation
- **AND** mutation through either property alias remains visible on that element
- **AND** clearing one structure does not change the other

#### Scenario: Property assignment
- **GIVEN** a selected characteristic element with additional properties
- **WHEN** application code assigns a different structure to AdditionalProperties
- **THEN** assignment fails and the existing structure retains its contents
