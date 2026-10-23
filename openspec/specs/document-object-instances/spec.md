# document-object-instances Specification

## Purpose
Execute supported in-memory document objects with acquired metadata, native handlers and explicit data-adapter boundaries.

## Requirements

### Requirement: Explicit document number assignment
Document objects SHALL expose УстановитьНовыйНомер / SetNewNumber with zero arguments or one String prefix argument. Before document selection the operation SHALL fail. For a selected document the original instance-bound ПриУстановкеНовогоНомера handler, when present, SHALL receive the standard-processing flag initially true and the supplied prefix, initially empty when omitted. Its returned parameter values SHALL determine whether standard allocation occurs and which prefix it uses. Cancelling standard processing SHALL preserve changes made by the handler and SHALL NOT invoke the allocation adapter.

#### Scenario: A handler cancels standard allocation
- **WHEN** the handler assigns Number to manual and sets standard processing to false
- **THEN** the method preserves manual and does not request a number from the data adapter

#### Scenario: A handler replaces the prefix
- **WHEN** the handler replaces an input prefix with H- and retains standard processing
- **THEN** allocation receives H- and the document's current date
- **AND** the returned number is assigned through the existing document-number qualifier handling

#### Scenario: A numeric document uses a single-parameter handler
- **GIVEN** a numeric document declares ПриУстановкеНовогоНомера with only the standard-processing parameter
- **WHEN** either method alias is called, including with a String prefix
- **THEN** the handler receives exactly its single Boolean parameter
- **AND** cancellation preserves its manually assigned numeric Number without requiring an allocation adapter

#### Scenario: A String document also omits the event prefix parameter
- **GIVEN** a String-number document declares only the standard-processing event parameter
- **WHEN** either method alias is called, including with prefix P-
- **THEN** the handler receives exactly one parameter
- **AND** cancellation preserves its assigned String Number without requiring an allocation adapter

#### Scenario: Initialization cannot reserve a number
- **WHEN** the object-module body invokes SetNewNumber before selection
- **THEN** the call fails without allocating a number
- **AND** the first successful call after creation receives the first available number

### Requirement: Session-bound allocation adapter
Standard document-number allocation SHALL use the current session's data adapter with the document metadata, current principal, current date, final prefix and existing call context. Core runtime and VM SHALL NOT acquire database dependencies or a second allocator. An absent or unsupported adapter SHALL report an explicit error, rather than fabricate a number. Allocation against an actual infobase SHALL preserve the acquired numbering scope, date period and prefix; unknown required settings SHALL cause an explicit failure. Supporting the handler and dispatch alone SHALL NOT be reported as completed live-infobase numbering.

#### Scenario: An adapter cannot allocate
- **WHEN** standard processing is enabled and the current data adapter does not support document-number allocation
- **THEN** the call fails explicitly and does not assign a fabricated number

#### Scenario: Native yearly scopes remain independent
- **WHEN** successive numbers are allocated with the measured Year settings for 2024, again 2024, 2025, an empty date, and again 2024 on another object
- **THEN** the observed numeric suffixes are 1, 2, 1, 1 and 3 respectively
- **AND** a separate P- prefix begins its own observed sequence

### Requirement: Acquired document numbering scope
The ORM document descriptor SHALL retain completely acquired numbering periodicity and the configuration object identity of its numerator. The five natively acquired periodicities SHALL remain distinct: Nonperiodical, Year, Quarter, Month and Day. An unknown or incomplete scope SHALL remain unknown; a known scope with no numerator SHALL remain distinguishable from unknown acquisition. The data adapter SHALL receive this descriptor without introducing SDK types or database dependencies into core runtime or VM.

#### Scenario: A yearly document has no configured numerator
- **WHEN** the acquired descriptor declares Year and a proven empty numerator reference
- **THEN** the ORM descriptor retains Year and a known absence of a numerator
- **AND** this state is not represented as unknown numbering settings

#### Scenario: Two documents share a configured numerator
- **WHEN** two acquired descriptors reference the same nonempty numerator object GUID
- **THEN** both ORM descriptors preserve that same identity
- **AND** it is not replaced by either document's own object or generated type identity

#### Scenario: Unknown acquisition does not invent a scope
- **WHEN** the SDK does not provide complete numbering settings
- **THEN** the ORM descriptor retains unknown settings instead of assuming Nonperiodical or no numerator

### Requirement: New document reference assignment
New document objects SHALL support ПолучитьСсылкуНового / GetNewObjectRef with zero arguments and УстановитьСсылкуНового / SetNewObjectRef with one argument. The initial future reference SHALL be the document's typed empty reference. Assignment SHALL accept only references of that same document and configuration, including an empty reference. Rejected assignments SHALL preserve the preceding future reference. Assigning a future reference SHALL NOT change the new object's empty Ссылка property or write a document row.

#### Scenario: An explicit document UUID becomes the future reference
- **WHEN** a reference with an explicit UUID is assigned to a new object of that document
- **THEN** both getter aliases return that exact reference
- **AND** the object's Ссылка remains empty and a second object's future reference remains empty

#### Scenario: Invalid inputs do not overwrite a future reference
- **WHEN** Undefined, Null, Number, String, Boolean, UUID, Array, a document object, a reference of another document or a catalog reference is assigned
- **THEN** assignment is rejected and the previous future reference remains unchanged

#### Scenario: Object-module initialization cannot access a future reference
- **WHEN** the object-module body invokes either future-reference method before document selection
- **THEN** the call is rejected
- **AND** after creation the getter returns the document's typed empty reference

### Requirement: Acquired document register record collections
Document objects SHALL expose Движения / RegisterRecords as a read-only collection of the document's acquired associated registers. An unknown association list SHALL NOT be treated as an empty list. A newly created document's collection SHALL contain one independent record set per associated register, without reading or writing business rows during creation. The collection SHALL preserve its identity across object-module initialization.

#### Scenario: Empty acquired associations are genuinely empty
- **WHEN** a document with an acquired empty association list is created
- **THEN** its collection type is the native register record collection and both Count aliases return zero
- **AND** iteration yields no elements and assigning the property is rejected

#### Scenario: A subordinate information register has its own empty record set
- **WHEN** a new document is associated with РасписаниеЗагрузкиРесурсов
- **THEN** the collection contains one record set with the exact configuration-owned information-register record-set type
- **AND** retrieving it by register name or index yields the same record set
- **AND** it initially contains zero records and its Записывать flag is false

#### Scenario: Native information-register English members agree
- **WHEN** the same new information-register record set is queried using Count and Write
- **THEN** they report the same zero record count and false flag as Количество and Записывать

#### Scenario: Document record-set flags are isolated
- **WHEN** one document's record-set Записывать flag is set to true
- **THEN** the same record set retrieved by index reports true
- **AND** a separately created document's record-set flag remains false

#### Scenario: Object-module collection identity is retained
- **WHEN** the object-module body retains Движения in a module variable
- **THEN** that retained collection equals the selected document's collection after creation
- **AND** the RegisterRecords alias returns the same collection

#### Scenario: Body record-set mutation remains on the selected document
- **WHEN** the object-module body retains its associated record set and sets Записывать to true
- **THEN** both the retained set and the selected document's set report true and have the same identity
- **AND** assigning false through Write after creation is reflected through Записывать

#### Scenario: Unavailable associations are explicit
- **WHEN** Движения is accessed without a complete acquired association list
- **THEN** access reports unavailable metadata rather than returning an empty collection or Undefined

### Requirement: Document tabular section instances
Document objects SHALL expose their acquired own tabular sections in their object-module context and through object properties. Each document instance SHALL own independent rows. Section and row values SHALL retain their concrete configuration-owned types rather than exposing a value table as the section itself. Unknown acquisition SHALL remain distinguishable from an acquired empty collection. The measured Add, Count, Unload, Delete, Clear and LineNumber English aliases SHALL behave identically to their Russian names.

#### Scenario: New document sections are independent
- **WHEN** two documents with an acquired tabular section are created and a row is added to the first section
- **THEN** the second section contains zero rows
- **AND** the first row has line number 1 and the measured Boolean and Number initial values false and zero

#### Scenario: Deleted rows are renumbered
- **WHEN** the first row of a three-row section is deleted
- **THEN** two rows remain and the first remaining row has line number 1
- **AND** clearing the section leaves the document property referring to a section with zero rows

#### Scenario: English names preserve the native section behavior
- **WHEN** the native creation, unloading, deletion and isolation probe uses Add, Count, Unload, Delete, Clear and LineNumber
- **THEN** every observation equals the corresponding Russian-name observation

#### Scenario: Object-module sections precede the selected document instance
- **WHEN** the object-module body retains its section in a module variable and adds a row to it
- **THEN** creation completes with a new empty section on the document
- **AND** the retained section still contains one row and differs from the document section

#### Scenario: Line numbers are read-only and row identity is stable
- **WHEN** a row is added and its LineNumber is assigned explicitly
- **THEN** the assignment is rejected and its line number remains 1
- **AND** retrieving that row by index yields the same row identity

### Requirement: Filtered tabular section unloading
A document tabular section SHALL support the measured Unload call with a Structure filter. The result SHALL be a separate value table containing rows matching every supplied field. Its standard line-number column SHALL be named LineNumber, followed by acquired own attributes in declaration order. Mutating result cells SHALL NOT change source cells.

#### Scenario: Original UNF filter matches all supplied fields
- **WHEN** a section contains rows with Boolean/Number pairs true/2, true/3 and false/2 and Unload receives the filter true/2
- **THEN** the result is a value table containing one row and the three measured columns LineNumber, ТипНоменклатурыЗапас and НомерВариантаКП
- **AND** changing its НомерВариантаКП cell to 9 leaves the source cell equal to 2

### Requirement: Acquired document object metadata
Document creation SHALL use numbering qualifiers and owned attribute types acquired from configuration metadata. Unknown descriptors SHALL remain distinguishable from known empty declarations and SHALL NOT be replaced by fabricated defaults.

#### Scenario: Known string numbering
- **WHEN** the source supplies string numbering with length nine
- **THEN** the object uses those qualifiers

#### Scenario: Unknown document settings
- **WHEN** the source cannot acquire settings required for object creation
- **THEN** creation fails explicitly rather than assuming string numbering

### Requirement: New document object
Document managers SHALL expose СоздатьДокумент / CreateDocument with zero arguments. The method SHALL return an object of the exact owning document type without writing a database row. Existing same-name configuration exports SHALL retain their behavior and arity.

#### Scenario: Measured initial string-number document
- **WHEN** a supported string-number document is created
- **THEN** Number is empty, Date is Date(1,1,1), Ref is empty and IsNew is true
- **AND** DeletionMark and Posted are false

#### Scenario: Aliases and argument counts
- **WHEN** code calls the platform creation method using an alias in another case
- **THEN** it creates the same document type
- **AND** passing any arguments fails

### Requirement: Concrete document module initialization
Each document object SHALL have its own object-module instance. Its body SHALL execute once before creation returns. During measured string-number initialization, reads of Number, Date, Ref, DeletionMark and Posted SHALL succeed; writes of Number and Date and IsNew SHALL fail. Failed initialization SHALL not publish a usable object.

#### Scenario: Body property access
- **WHEN** the native-equivalent object body catches Number assignment and IsNew failures
- **THEN** the body finishes and subsequent exports can read its recorded results

#### Scenario: Distinct object state
- **WHEN** two document objects mutate module variables
- **THEN** their object-module state remains independent
- **AND** repeated calls on one object preserve its state

#### Scenario: Session isolation
- **WHEN** another State creates the same document type
- **THEN** its module state and host data remain isolated from the first State

#### Scenario: Unhandled body failure
- **WHEN** document body initialization raises an unhandled error
- **THEN** CreateDocument fails without returning the partially initialized object

### Requirement: Measured document standard assignments
After successful creation, standard property assignment SHALL follow acquired qualifiers and native conversion behavior. Setting Posted or DeletionMark in memory SHALL NOT imply database persistence or document posting.

#### Scenario: String numbering length nine
- **WHEN** a string-number document of length nine receives Number="123456789012"
- **THEN** Number becomes "123456789"
- **AND** assigning Undefined or Null makes Number empty

#### Scenario: Numeric numbering length nine
- **WHEN** a numeric-number document of length nine receives Number="123456789012"
- **THEN** Number becomes 999999999
- **AND** assigning Undefined or Null makes Number zero

#### Scenario: Date conversion
- **WHEN** a created document receives Date="20260102"
- **THEN** Date becomes Date(2026,1,2)
- **AND** assigning Undefined or Null makes Date equal Date(1,1,1)

#### Scenario: In-memory state flags
- **WHEN** code assigns Posted=true and DeletionMark=true after creation
- **THEN** those properties read as true
- **AND** no database row is written by those assignments

#### Scenario: Disabled numbering is not a zero-length truncation
- **WHEN** the measured document with string NumberLength=0 receives Number="123456789012"
- **THEN** Number retains the complete string
- **AND** the assignment is not rejected merely because numbering is disabled

### Requirement: Document filling event
Заполнить / Fill SHALL take one required arbitrary value and invoke the original document object-module filling handler with that value. The supported original two-parameter handler SHALL initially receive StandardProcessing=true. Filling SHALL obey acquired attribute filling settings instead of unconditionally copying every matching standard property. It SHALL NOT write a database row.

#### Scenario: Original two-parameter filling handler
- **WHEN** Fill receives a structure and the document declares the supported two-parameter ОбработкаЗаполнения handler
- **THEN** the handler receives that structure and StandardProcessing=true
- **AND** its object-property assignments are visible after the method returns

#### Scenario: Arbitrary filling data
- **WHEN** Fill receives Undefined, Null or Number
- **THEN** the original value reaches the handler without conversion to a structure

#### Scenario: Measured minimal metadata does not fill standard properties
- **WHEN** the measured minimal document handler sets Number="handler"
- **AND** Fill receives a structure containing Number="filled" and Date=Date(2026,1,2)
- **THEN** Number remains "handler" and Date remains empty after filling

### Requirement: Instance-bound platform handler calls
Runtime components SHALL be able to invoke a non-exported handler on an initialized object-module instance through the facade. The call SHALL retain that instance's receiver and variables and return final parameter values. Ordinary object-method calls SHALL continue to reject non-exported methods. Foreign-State instances SHALL remain rejected.

#### Scenario: Platform handler updates one document
- **WHEN** a component invokes a private handler on the first of two initialized document instances
- **THEN** the handler changes only the first instance's module variables and receiver
- **AND** final parameter values are returned to the component

#### Scenario: Private handler is not a public method
- **WHEN** ordinary BSL code calls that handler through the document object
- **THEN** the call fails because the handler is not exported

#### Scenario: Foreign-State platform handler call
- **WHEN** a component invokes the handler using an instance owned by another State
- **THEN** the call fails before handler execution

### Requirement: Compiled handler parameter inspection
The facade SHALL allow a runtime component to inspect the declared parameter count of a function in an already compiled configuration module, including non-exported handlers. An absent function SHALL return no count. Inspection SHALL not execute module bodies, mutate object state or weaken ordinary export checks.

#### Scenario: Two- and three-parameter handlers
- **WHEN** the compiled module contains the original two-parameter handler or the measured three-parameter handler
- **THEN** inspection returns two or three respectively
- **AND** no handler or module body executes

#### Scenario: Absent handler
- **WHEN** the requested function is absent from the compiled module
- **THEN** inspection returns no count

### Requirement: Measured three-parameter filling handler
A supported three-parameter filling handler invoked by Fill SHALL receive the original filling data, Undefined as FillingText and true as the initial StandardProcessing value.

#### Scenario: Filling with a structure
- **WHEN** Fill invokes the measured three-parameter handler with a structure
- **THEN** FillingText is Undefined and StandardProcessing is initially true

### Requirement: Acquired owned document attribute filling
The document component SHALL retain acquired owned attribute types, FillFromFillingValue and typed FillValue, preserving unknown acquisition states. Creating a document SHALL use initial type defaults rather than applying filling defaults immediately. Standard filling SHALL respect handler results and configured filling rules.

#### Scenario: Creation and filling defaults differ
- **WHEN** the measured document has owned string attributes with FillValue="default"
- **THEN** creation leaves them empty
- **AND** standard filling of empty fields without a matching source value applies "default"

#### Scenario: Handler results are preserved
- **WHEN** the handler sets a string attribute to "handler"
- **AND** the source structure supplies "filled"
- **THEN** standard filling preserves "handler"

#### Scenario: Configured source filling
- **WHEN** the handler leaves the measured attributes empty and the source structure supplies "filled" for both
- **THEN** the attribute permitting source filling becomes "filled"
- **AND** the attribute forbidding source filling receives its configured "default"

#### Scenario: Standard filling is disabled
- **WHEN** the handler sets StandardProcessing=false and leaves those attributes empty
- **THEN** neither source values nor configured filling defaults are applied

#### Scenario: Handler is absent
- **WHEN** the document has no filling handler
- **THEN** standard filling still follows acquired settings
- **AND** handler absence is not an invocation error

#### Scenario: Handler replaces filling data
- **WHEN** the handler replaces FillingData with a different structure
- **THEN** standard filling uses the returned structure
- **AND** the original caller structure retains its original fields

#### Scenario: Handler removes filling data
- **WHEN** the handler replaces FillingData with Undefined or an empty structure
- **THEN** standard filling applies configured defaults to empty attributes without matching source fields

#### Scenario: Handler mutates a shared source structure
- **WHEN** the handler changes a field of the supplied structure
- **THEN** standard filling uses the changed field
- **AND** the caller observes that field change

### Requirement: Document exchange and additional properties
Document objects SHALL expose ОбменДанными / DataExchange and ДополнительныеСвойства / AdditionalProperties. After creation, exchange loading SHALL initially be false and recipient automatic filling SHALL initially be true. Each object SHALL retain its own exchange state and additional property structure. These properties SHALL NOT be replaceable by assignment.

#### Scenario: Exchange access during document initialization
- **WHEN** the original document object-module body accesses DataExchange
- **THEN** its value is Undefined because the document is not yet selected
- **AND** access to nested exchange properties fails
- **AND** catching those failures allows initialization to continue

#### Scenario: Additional properties during document initialization
- **WHEN** the object-module body reads and mutates AdditionalProperties
- **THEN** it can use the initially empty structure
- **AND** the mutations remain visible after creation

#### Scenario: Independent exchange and additional properties
- **WHEN** code changes Load, Recipients.AutoFill and an additional property on one created document
- **THEN** those values remain on that document
- **AND** a second document and a document in another State retain their own values

#### Scenario: Property aliases and replacement
- **WHEN** code uses either property alias in a different case
- **THEN** it accesses the same object-owned value
- **AND** assignment replacing DataExchange or AdditionalProperties fails

### Requirement: Measured boolean and datetime document attributes
Document objects SHALL support acquired single-type Boolean and datetime attributes in addition to the measured string attributes. Their initial values SHALL be false and the empty date. Their assignments and filling SHALL follow the measured native conversions, using acquired typed filling defaults. Unsupported types or qualifiers SHALL remain explicit failures.

#### Scenario: Boolean and datetime attributes during initialization
- **WHEN** the object-module body reads the measured attributes
- **THEN** Boolean is false and datetime is empty
- **AND** assignments before document selection fail

#### Scenario: Typed standard filling defaults
- **WHEN** empty Boolean and datetime attributes are filled without matching source fields
- **THEN** the acquired defaults true and Date(2026,1,2,3,4,5) are applied

#### Scenario: Existing Boolean and datetime values
- **WHEN** the Boolean attribute is true and datetime is nonempty before filling
- **THEN** standard filling preserves those values
- **AND** a false Boolean remains eligible for filling on a later call

#### Scenario: Explicit empty typed source fields
- **WHEN** matching source fields contain Undefined or Null
- **THEN** the Boolean attribute becomes false and datetime becomes empty
- **AND** those present fields do not select configured filling defaults

#### Scenario: Native attribute conversions
- **WHEN** code assigns 17 or the measured text "Истина" to the Boolean attribute
- **THEN** it becomes true
- **AND** assigning Undefined or Null makes it false
- **AND** assigning "20260102030405" to the datetime attribute preserves the parsed date and time
- **AND** assigning invalid date text makes datetime empty

### Requirement: Measured catalog-reference document attributes
Document objects SHALL support the measured single catalog-reference attribute and two-type catalog-reference/string attribute using exact types from the owning configuration. They SHALL NOT obtain references by reading or writing database rows during creation or assignment. Unsupported declarations and conversions SHALL remain explicit failures.

#### Scenario: Reference and composite initial values
- **WHEN** the measured document is created
- **THEN** its single reference attribute is an empty reference of the declared catalog type
- **AND** its reference/string attribute is Undefined
- **AND** those values can be read during the object-module body while assignments still fail

#### Scenario: Typed reference assignment
- **WHEN** a reference of the declared catalog type is assigned to either attribute
- **THEN** its exact type and UUID are retained
- **AND** standard filling preserves a nonempty reference

#### Scenario: Primitive reference conversion
- **WHEN** Undefined, Null, Number, Boolean or String is assigned to the measured single reference attribute
- **THEN** it becomes the empty reference of the declared type
- **AND** the measured composite attribute converts Number and Boolean to a qualified string
- **AND** Undefined or Null makes the composite attribute Undefined

#### Scenario: Reference filling defaults and present source fields
- **WHEN** standard filling has no matching source field for the measured composite attribute
- **THEN** its acquired string default is applied
- **AND** a present source field containing Undefined or Null remains distinguishable from an absent field

#### Scenario: Explicit empty composite values are preserved
- **WHEN** the measured reference/string attribute already contains an empty reference or an empty string
- **THEN** standard filling preserves that value
- **AND** it does not replace it with the acquired string filling default

### Requirement: Measured enumeration document attributes
Document objects SHALL support an acquired single enumeration-reference attribute and the measured enumeration-reference/string union using the concrete reference types of their configuration. Creation and assignment SHALL use immutable values without reading database rows.

#### Scenario: Initial and typed values
- **WHEN** the measured document is created
- **THEN** its single enumeration attribute is its own typed empty reference
- **AND** its enumeration/string attribute is Undefined
- **AND** assigning a member of the declared enumeration retains its concrete type and identity

#### Scenario: Measured primitive and system-enumeration conversion
- **WHEN** Undefined, Null, String, Number, Boolean or a system-enumeration value is assigned to the measured single enumeration attribute
- **THEN** it becomes its own empty reference
- **AND** the union converts Number, Boolean and the measured foreign system-enumeration value to a qualified string
- **AND** Undefined or Null makes the union Undefined

#### Scenario: Present source versus missing source
- **WHEN** standard filling receives a present source field with Undefined or an empty enumeration reference
- **THEN** that field is distinguished from a missing source field
- **AND** the measured union with a missing field receives its acquired string default

#### Scenario: Existing enumeration values during filling
- **WHEN** the single enumeration attribute already contains a member
- **THEN** standard filling preserves it even when the source contains a different member
- **AND** a single empty reference remains eligible for filling
- **AND** the union preserves its existing member, empty reference or empty string

### Requirement: Measured document-reference attributes
Document objects SHALL support the acquired single document-reference attribute and the measured document-reference/string union using exact configuration-owned reference types. Creation and assignment SHALL NOT read or write document rows.

#### Scenario: Initial document references
- **WHEN** the measured document is created
- **THEN** its single document-reference attribute is an empty reference of the declared document type
- **AND** its document-reference/string attribute is Undefined

#### Scenario: Typed document reference assignment
- **WHEN** a reference of the declared document type is assigned or supplied to standard filling
- **THEN** its concrete type and UUID are preserved

#### Scenario: Measured incompatible input
- **WHEN** Undefined, Null, Number, Boolean, String or the measured foreign system-enumeration value is assigned to the single document-reference attribute
- **THEN** it becomes its own typed empty reference
- **AND** the union converts Number, Boolean and the measured system-enumeration value to a qualified string
- **AND** Undefined or Null makes the union Undefined

#### Scenario: Missing versus present source
- **WHEN** the measured document-reference/string source field is missing
- **THEN** its acquired string filling default is applied
- **AND** a present Undefined or empty reference is distinguished from a missing source field

#### Scenario: Existing document-reference values during filling
- **WHEN** the single reference attribute already contains a nonempty document reference
- **THEN** standard filling preserves it even when the source contains a different UUID
- **AND** an empty single reference remains eligible for filling
- **AND** the union preserves an existing empty reference or empty string

### Requirement: Document manager reference construction
A document manager SHALL expose GetRef under its Russian and English names with zero or one argument. An explicit UUID SHALL be retained, including nil. An omitted or Undefined argument SHALL produce a new nonempty reference of that document type. A string or Null SHALL be rejected. Reference construction SHALL NOT read or write document rows.

#### Scenario: Explicit identity
- **WHEN** GetRef receives the measured UUID
- **THEN** the resulting reference returns that UUID and has the manager's document reference type
- **AND** a nil UUID produces an empty reference

#### Scenario: Generated identity and aliases
- **WHEN** GetRef or ПолучитьСсылку receives no argument or Undefined
- **THEN** it returns a nonempty reference of the manager's document type

#### Scenario: Invalid UUID representation
- **WHEN** GetRef receives a UUID-shaped string or Null
- **THEN** it fails instead of parsing the string or creating an empty reference

### Requirement: Measured mixed reference attributes
Document objects SHALL support the measured document/catalog reference union and the same union with a qualified String. Every reference declaration SHALL resolve to its exact configuration-owned type; unknown declarations SHALL remain explicit failures.

#### Scenario: Mixed reference identities
- **WHEN** either declared reference is assigned or supplied to filling
- **THEN** its concrete type and UUID are retained, including nil UUIDs
- **AND** the initial value of the mixed reference attribute is Undefined

#### Scenario: No declared string conversion
- **WHEN** the reference-only union receives Undefined, Null, String, Number, Boolean or the measured system-enumeration value
- **THEN** its value becomes Undefined

#### Scenario: Optional string member
- **WHEN** the union also declares String
- **THEN** the measured Number, Boolean and system-enumeration inputs convert to its qualified string
- **AND** an absent source field selects the acquired string default
- **AND** present Undefined or Null remains distinct from an absent field

#### Scenario: Complete UNF event basis declaration
- **WHEN** the original event-basis DefinedType declares 54 document references and two catalog references
- **THEN** assignment and filling retain every declared concrete reference type and UUID, including nil
- **AND** no declaration member is discarded because of its position or the size of the set

### Requirement: Measured ValueStorage document attributes
A document attribute declaring ValueStorage SHALL use the existing ORM value storage object and its detached value graph. It SHALL NOT expose an untyped Undefined initial value or wrap primitive assignments as stored contents.

#### Scenario: Initial and assigned storage
- **WHEN** a document with the measured ValueStorage attribute is created
- **THEN** its attribute is a ValueStorage whose Get returns Undefined
- **AND** assignment of a ValueStorage containing String retains those stored contents
- **AND** assignment of the measured Undefined, Null, String, Number or Boolean input yields storage containing Undefined

#### Scenario: Standard filling of initial storage
- **WHEN** standard Fill receives any of the measured inputs, including storage containing String, on a newly created object
- **THEN** the attribute remains storage containing Undefined

### Requirement: Fixed-length string document numbers
The document number settings SHALL distinguish measured fixed-length String declarations from variable-length String declarations. Initial values and accepted assignments SHALL be truncated or padded with spaces to the configured UTF-16 length, matching the native oracle. Existing variable-length declarations SHALL preserve their behavior.

#### Scenario: Native fixed-length initialization and assignment
- **WHEN** the measured document declares fixed-length String Number of length nine
- **THEN** its initial Number is nine spaces
- **AND** assignment of short String, Number, Boolean, Undefined or Null produces the measured padded String
- **AND** a longer String is truncated to nine characters

### Requirement: Acquired document attribute assembly
The factory adapter SHALL assemble document attributes from the same acquired SDK snapshot after named-type expansion. Each declared reference type GUID SHALL resolve to its acquired owner and exact ORM type. Typed filling defaults SHALL use the existing transferable value graph. Unknown declarations or defaults SHALL remain explicitly unavailable rather than being replaced by guessed values. Attaching attributes to a catalog document SHALL retain configuration properties, other objects and enumeration identities.

#### Scenario: Owner-bound assembly
- **WHEN** the host attaches acquired attributes to a known document before creating sessions
- **THEN** the catalog retains their complete order and filling settings
- **AND** an unknown owner or an owner of another kind is rejected

#### Scenario: Full event basis
- **WHEN** the SDK snapshot expands the acquired event basis definition
- **THEN** the adapter retains every one of its 56 exact reference types
- **AND** it does not interpret the named declaration as an empty type
