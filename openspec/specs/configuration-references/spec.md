# configuration-references Specification

## Purpose
Define configuration-owned reference types, manager receiver contexts, native
reference identity, and session-bound resolution of predefined records.

## Requirements

### Requirement: Host-declared configuration value types
The facade SHALL accept immutable value type descriptions owned by the engine.
TypeOf and type-name lookup SHALL return the same declared type. Declarations
SHALL provide explicit lookup names separately from the printed representation.
Existing native and static component type resolution SHALL retain its behavior.
Conflicting declarations SHALL fail engine construction.

#### Scenario: Qualified catalog reference type
- **WHEN** a catalog reference belongs to catalog C
- **THEN** TypeOf prints C's acquired name
- **AND** its declared Russian and English qualified names resolve to that type
- **AND** the printed name alone does not become an implicit lookup alias

#### Scenario: Type lifetime
- **WHEN** a value type is retained after its engine is dropped
- **THEN** its immutable description remains valid without leaked static storage

#### Scenario: Background engine declarations
- **WHEN** a background worker reconstructs its engine from the parent's configuration recipe
- **THEN** it retains the parent's immutable value type declarations
- **AND** a transferred type value resolves in that catalog

#### Scenario: Conflicting declarations
- **WHEN** two host value types claim the same lookup name
- **THEN** engine construction fails instead of choosing one by registration order

### Requirement: Explicit manager receiver context
A configuration manager module SHALL be able to call the platform methods
declared for its own receiver without qualifying that receiver. Only names
explicitly supplied by the host SHALL become receiver methods. The invocation
SHALL use the current State's ThisObject and existing method dispatch.
The receiver's platform method SHALL take priority over a global
common-module export with the same name.

#### Scenario: Own reference method
- **WHEN** a catalog manager module calls its declared EmptyRef method
- **THEN** the call reaches the current catalog manager in the same State

#### Scenario: No context in a different module
- **WHEN** another module calls that name without its own declaration or an ordinary visible function
- **THEN** it retains the existing undefined-function diagnostic

#### Scenario: Global export with the same name
- **WHEN** a global common module exports the name of a receiver method
- **THEN** the manager module calls its receiver method
- **AND** a module without that receiver context calls the global export

### Requirement: Catalog reference identity
Catalog EmptyRef SHALL return an unfilled reference with a nil UUID.
GetRef SHALL preserve an explicit UUID and create a new unique reference when
the UUID is omitted. References of the same catalog with the same UUID SHALL
compare equal. UUID retrieval SHALL return the UUID value, not a string.
Creation of a reference SHALL NOT read or write its catalog row.

#### Scenario: Nil UUID
- **WHEN** GetRef receives the nil UUID
- **THEN** its result equals EmptyRef and is not filled

#### Scenario: Repeated explicit UUID
- **WHEN** GetRef is called twice with the same non-nil UUID
- **THEN** the references compare equal and expose that UUID

#### Scenario: Omitted UUID
- **WHEN** GetRef is called twice without its optional argument
- **THEN** the resulting references are filled and distinct
- **AND** their UUIDs have version 1 as measured on native 1C 8.3.27.2342

#### Scenario: Typed query parameter
- **WHEN** a catalog reference is supplied as a query parameter
- **THEN** the database adapter preserves its reference type and UUID
- **AND** the query retains the current principal's rights checks

#### Scenario: Undefined optional UUID
- **WHEN** GetRef receives explicit Undefined
- **THEN** it creates a new filled reference as when the argument is omitted

#### Scenario: Invalid UUID argument
- **WHEN** GetRef receives a number, UUID string or Null
- **THEN** it rejects the argument instead of coercing it

#### Scenario: Typed query result reference
- **WHEN** a supported catalog reference is returned by a query
- **THEN** the adapter materializes a reference of that acquired catalog
- **AND** its UUID preserves the native physical reference identity

### Requirement: Component presentation failures
The object protocol SHALL provide a fallible presentation hook separately
from diagnostic Display. Existing component objects SHALL retain their current
presentation by default. A catalog's empty reference SHALL have an empty
presentation. A reference requiring unavailable data access SHALL report an
explicit error instead of exposing diagnostic text as a platform presentation.

#### Scenario: Empty reference presentation
- **WHEN** String receives a catalog EmptyRef
- **THEN** it returns the empty string

#### Scenario: Presentation without the required adapter
- **WHEN** a filled catalog reference has no supported presentation data access
- **THEN** formatting reports an error instead of inventing its presentation

### Requirement: Component access to the host clock
The VM SHALL attach the current State's host clock to component contexts
without replacing a host-injected Clock. Reduced contexts without
this capability SHALL report CapabilityMissing with Capability Clock.

#### Scenario: Reference generation with a host clock
- **WHEN** the host supplies a fixed clock and requests several new catalog references
- **THEN** version 1 UUID timestamps advance without duplicate identifiers
- **AND** the ordinary UUID constructor retains version 4 generation

#### Scenario: Clock unavailable
- **WHEN** a reduced context requests its clock without a supplied capability
- **THEN** the request fails with CapabilityMissing Clock

### Requirement: Typed empty document reference
A document manager SHALL expose zero-argument ПустаяСсылка / EmptyRef through its finite receiver method context. It SHALL return an immutable empty reference of its owning document, not a catalog reference or Undefined. Type names, presentation, UUID, emptiness and equality SHALL match the preserved native measurement.

#### Scenario: Own manager empty reference
- **WHEN** an original document manager module calls bare EmptyRef
- **THEN** it obtains an own document reference with nil UUID and unfilled state
- **AND** repeated own empty references compare equal without reading a document record

#### Scenario: Distinct reference families
- **WHEN** empty document and catalog references are compared or their qualified types are resolved
- **THEN** their concrete types remain distinct
- **AND** Russian and English document-qualified aliases resolve to the same document type

### Requirement: Document reference transfer retains its type
The existing immutable value graph SHALL reconstruct an acquired document reference using its declared document type and UUID without transferring a live session or converting it to a catalog reference. Catalog-specific public decomposition helpers SHALL reject document references.

#### Scenario: Reconstructed empty reference
- **WHEN** an empty document reference is stored or transferred through the existing graph
- **THEN** it retains its owning document type, nil UUID and equality
- **AND** catalog-only decomposition does not classify it as a catalog reference

### Requirement: Acquired catalog predefined manager properties
An acquired catalog manager SHALL expose read-only properties named by its declared predefined items. The owning manager module SHALL resolve the same properties as bare context names through the existing engine, preserving local and module variable precedence. Unknown names SHALL retain existing diagnostics.

#### Scenario: Original manager reads its predefined item
- **WHEN** an original catalog manager reads an acquired predefined item by its symbolic name
- **THEN** bare and qualified access resolve the same owning catalog reference
- **AND** reading the property does not execute a rewritten version of the original source
- **AND** PredefinedValue of the same catalog path resolves the same session reference

#### Scenario: Ordinary variable takes precedence
- **WHEN** a local or module variable has the same name as a predefined manager property
- **THEN** the existing variable resolution priority is preserved

### Requirement: Predefined references use session data
Predefined property resolution SHALL use the actual record reference of the current information base through its session data adapter. A configuration predefined identity SHALL NOT be fabricated into a record UUID. The implementation SHALL preserve applicable existing rights and data-separation checks and SHALL NOT share live session connections through global or thread-local state.

#### Scenario: Resolve an existing predefined record
- **WHEN** the current session reads a predefined manager property backed by an existing record
- **THEN** its UUID and owning reference type match native 1C and the actual database record

#### Scenario: Session isolation
- **WHEN** different sessions resolve the same predefined symbolic name
- **THEN** each resolution uses that session's data adapter and information base

#### Scenario: Predefined data has not been created
- **WHEN** the declared predefined item has no record in the current information base
- **THEN** reading the manager property fails explicitly, matching the native probe
- **AND** no empty reference, metadata UUID or Undefined is substituted

### Requirement: Missing enumeration reference identity
A valid enumeration reference SHALL retain its type and UUID even when the current configuration value list does not contain that UUID. Such a nonempty reference SHALL remain filled, compare by type and identity, and survive value-graph capture and restoration. Named enumeration value lookup SHALL continue to use the declared list; accepting a reference SHALL NOT manufacture a named metadata value.

#### Scenario: Acquired absent filling value
- **WHEN** acquired document metadata contains a valid enumeration filling reference absent from the declared enumeration value list
- **THEN** acquisition SHALL preserve that reference instead of rejecting the catalog
- **AND** document filling SHALL transfer it as a nonempty reference of the same type

#### Scenario: Graph round trip
- **WHEN** a missing enumeration reference is captured and restored against the receiving catalog
- **THEN** its type, UUID, equality and filled state SHALL remain unchanged

#### Scenario: Strict malformed identity
- **WHEN** a payload has a malformed UUID or its owner is not an enumeration
- **THEN** materialization SHALL fail

#### Scenario: No fabricated named value
- **WHEN** code looks up an undeclared named enumeration member
- **THEN** it SHALL retain the existing missing-member failure

### Requirement: Missing enumeration reference presentation
A missing enumeration reference SHALL be presented with its acquired DBNames number and physical UUID bytes. The missing-object label SHALL follow the current session UI language. Rendering SHALL NOT mutate a shared configuration catalog or cache another session's language. Unknown presentation identity SHALL NOT be replaced with a fabricated database number.

#### Scenario: Native storage identity
- **WHEN** the native probe references an absent value of the enumeration stored in _Enum54
- **THEN** its presentation SHALL contain the measured `(54:84093272f97870604114bca67664d5d3)` identity

#### Scenario: Session language isolation
- **WHEN** the same acquired reference is presented by Russian and English UI sessions
- **THEN** their labels SHALL be Объект не найден and Object not found respectively
- **AND** formatting one session SHALL NOT change the other session's result

### Requirement: Contextual component presentation
The runtime SHALL provide an additive component presentation hook receiving the current call context, with a default preserving existing component presentation. String, Format, Message and component call-context formatting of extension values SHALL use the current presentation context. Diagnostic display SHALL remain separate. The facade SHALL expose the hook's required types without requiring implementation-crate imports.

#### Scenario: Existing component
- **WHEN** a component does not override contextual presentation
- **THEN** user-visible formatting SHALL retain its previous presentation result

#### Scenario: Missing presentation context
- **WHEN** a missing enumeration reference lacks an acquired storage number or current UI language
- **THEN** user-visible formatting SHALL fail explicitly
- **AND** capture, restoration and equality SHALL remain available without that formatting context

#### Scenario: Current state context
- **WHEN** two States format the same immutable acquired reference with different UI languages
- **THEN** each call SHALL use the invoking State's language
- **AND** the shared materializer SHALL NOT retain either State

#### Scenario: Facade UUID validation
- **WHEN** a configuration component validates a reference UUID
- **THEN** the facade SHALL expose the existing canonical UUID parser and formatter without an implementation-crate dependency

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

### Requirement: Acquired catalog owner collection
The configuration catalog SHALL retain the acquired optional ordered owner-ID collection. Unknown owners SHALL remain distinct from a known empty collection. The database adapter SHALL use the existing SDK acquisition rather than infer owners from names or SQL rows.

#### Scenario: Known and unknown owner acquisition
- **WHEN** the SDK supplies a catalog's owner IDs
- **THEN** the ORM SHALL retain every acquired ID in order
- **WHEN** owner acquisition is unknown
- **THEN** accessing Owner SHALL fail explicitly without preventing unrelated object operations

### Requirement: Native new catalog object Owner
New catalog objects SHALL expose Владелец and Owner as aliases. With one acquired catalog owner, the initial value SHALL be its empty reference. With multiple acquired owners, the initial value SHALL be Undefined. With a known empty owner collection, reading SHALL return Undefined and assignment SHALL fail.

#### Scenario: Single owner
- **WHEN** a new object has exactly one acquired owner
- **THEN** its object-module body SHALL read the typed empty owner reference
- **AND** assignment during that body SHALL fail before object selection
- **AND** subsequent assignment of a compatible reference SHALL retain its identity, while an incompatible string SHALL become the typed empty reference

#### Scenario: Multiple owners
- **WHEN** a new object has two acquired catalog owners
- **THEN** its initial Owner SHALL be Undefined
- **AND** references of either acquired owner SHALL be accepted
- **AND** an incompatible string SHALL produce Undefined

#### Scenario: Object isolation
- **WHEN** one new object receives an owner reference
- **THEN** another object or Session SHALL retain its own initial value
- **AND** the assignment SHALL NOT write a database row

### Requirement: Complete applied catalog common attributes
The configuration catalog SHALL retain acquired applicable common attributes separately from own attributes. Explicit acquired inclusion or exclusion SHALL override the acquired automatic default for the measured catalog modes. Unknown collection membership or usage SHALL NOT be silently discarded or represented by a known empty or complete partial collection.

#### Scenario: Explicit inclusion
- **WHEN** automatic use is disabled and Content explicitly includes a catalog
- **THEN** that catalog's object receiver SHALL expose the included common attribute name
- **AND** the own Attributes collection SHALL remain unchanged

#### Scenario: Automatic inclusion and explicit exclusion
- **WHEN** automatic use is enabled and Content explicitly excludes one catalog
- **THEN** a catalog without an override SHALL expose the common attribute
- **AND** the excluded catalog SHALL reject access to that attribute

#### Scenario: Unknown acquisition
- **WHEN** authoritative collection membership or applicable usage is unknown
- **THEN** the host SHALL receive an explicit acquisition or access error
- **AND** the ORM SHALL NOT claim it acquired a complete partial collection

### Requirement: Measured common string attribute object state
The included string common attribute SHALL be case-insensitive, have the acquired string length, and keep its value in the individual catalog object. Its initial value SHALL be an empty string readable during object-module initialization; assignment SHALL require a selected object. Unsupported common attribute types SHALL fail explicitly until implemented from platform evidence.

#### Scenario: Body and subsequent assignment
- **WHEN** the included НаименованиеЯзык1 has variable string length 150
- **THEN** the module body SHALL read an empty string and its assignment SHALL fail
- **AND** after creation a number assignment SHALL produce its string representation and a 151-character ASCII string SHALL truncate to 150 characters

#### Scenario: Instance isolation
- **WHEN** one object assigns the common attribute
- **THEN** another object or Session SHALL keep its own initial value
- **AND** assignment SHALL NOT write a database row

### Requirement: Acquired catalog hierarchy kind
The configuration catalog SHALL preserve the acquired optional hierarchy kind separately from its hierarchy flag. Unknown kind or flag SHALL fail explicitly when folder creation requires them and SHALL NOT alter existing CreateItem behavior.

#### Scenario: Unknown folder settings
- **WHEN** folder creation is requested without known hierarchy settings
- **THEN** creation SHALL fail explicitly without fabricating a folder
- **AND** existing CreateItem behavior SHALL remain available

### Requirement: Native catalog folder factory
In the absence of an existing same-name manager export, catalog managers SHALL expose zero-argument СоздатьГруппу/CreateFolder through qualified calls and the existing own-manager receiver context. For folders-and-items kind, the returned object's IsFolder SHALL follow the acquired hierarchy flag. For item-only kind, the operation SHALL fail with either hierarchy flag. New-object memory SHALL remain isolated and creation SHALL NOT persist a row. Previously callable manager exports SHALL retain their behavior and argument count.

#### Scenario: Hierarchical folders-and-items
- **WHEN** CreateFolder is called for a hierarchical folders-and-items catalog
- **THEN** its module body SHALL observe IsFolder false and reject assigning it
- **AND** the selected returned object SHALL observe IsFolder true, read-only through both aliases

#### Scenario: Disabled hierarchy
- **WHEN** CreateFolder is called with hierarchy disabled and folders-and-items kind
- **THEN** the module body and returned object SHALL observe IsFolder false

#### Scenario: Item-only kind
- **WHEN** CreateFolder is called with item-only kind and either hierarchy flag
- **THEN** the operation SHALL fail before constructing the object

#### Scenario: Existing item factory and argument count
- **WHEN** CreateItem is called with the same metadata
- **THEN** it SHALL retain its prior non-folder behavior
- **WHEN** CreateFolder receives an argument
- **THEN** the ordinary component arity error SHALL be returned

#### Scenario: Existing manager export
- **WHEN** a manager already exports CreateFolder with one argument
- **THEN** the export SHALL remain callable with that argument and retain its result
- **AND** the newly added native factory descriptor SHALL NOT replace it

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

### Requirement: Acquired user data separation snapshot
The host SHALL be able to attach an optional serialized user data separation snapshot to Principal. The runtime SHALL expose it as CurrentUser().РазделениеДанных / DataSeparation using a Structure value. Unknown snapshots SHALL fail explicitly rather than return an empty structure. Malformed snapshots with a root count other than one or a non-Structure root SHALL fail explicitly.

#### Scenario: Known empty snapshot
- **WHEN** the host supplies a graph containing one empty Structure
- **THEN** DataSeparation.Count() SHALL be zero

#### Scenario: Unknown snapshot
- **WHEN** the host has not supplied the snapshot
- **THEN** reading DataSeparation SHALL fail explicitly

### Requirement: User object structure lifetime
Repeated reads of DataSeparation on the same user object SHALL return the same mutable structure. Each CurrentUser call SHALL materialize an independent structure from the frozen Principal snapshot. Separate states SHALL never share mutable structures through that snapshot.

#### Scenario: Modify one returned structure
- **WHEN** code modifies a structure obtained from one current-user object
- **THEN** the same user object SHALL expose the change
- **AND** a subsequent CurrentUser call SHALL expose the original acquired snapshot

#### Scenario: Separate session states
- **WHEN** two states use the same immutable acquired Principal snapshot
- **THEN** mutation in one state SHALL NOT affect the other state

### Requirement: Business process and task reference declarations
The ORM catalog SHALL declare acquired business process and task reference types through the existing immutable configuration value-type mechanism. Russian and English qualified names SHALL resolve to the same declaration for a configured owner. The database adapter SHALL use acquired reference TypeIds to preserve these types in compound characteristic metadata.

#### Scenario: Native control reference identities
- **GIVEN** the independently compiled native business process and task controls
- **WHEN** their acquired metadata is projected into the ORM catalog
- **THEN** BusinessProcessRef and TaskRef identities resolve to their corresponding declarations
- **AND** the Russian qualified names resolve to those same declarations
- **AND** the two kinds remain distinct

#### Scenario: Missing reference declaration
- **GIVEN** a characteristic configured with a reference TypeId that has no supported acquired owner
- **WHEN** the database adapter projects its value types
- **THEN** projection fails explicitly without omitting that type

### Requirement: Acquired scheduled-job method name
Database-backed configuration metadata SHALL expose the method name of a scheduled job when its explicit module/procedure link has been acquired in the measured form. Russian ИмяМетода and English MethodName SHALL return the same read-only string consisting of the common-module name and procedure name separated by a dot.

#### Scenario: Native method link
- **WHEN** a scheduled job explicitly references a common module and procedure in the supported acquired form
- **THEN** both aliases return the qualified method name without the XML CommonModule prefix
- **AND** reading the property does not execute the job

#### Scenario: Invalid explicit target
- **WHEN** a recognized link targets a missing resource or a resource that is not the referenced common module
- **THEN** acquisition fails without substituting an inferred module name

#### Scenario: Unknown serialized form
- **WHEN** a scheduled-job description does not match a measured supported form
- **THEN** projection does not fabricate a method-name value

### Requirement: Native generic metadata type lookup names
The existing generic metadata value type SHALL be available through the Russian name ОбъектМетаданных and the English name MetadataObject. Existing lookup names and the distinct configuration-root type SHALL remain available.

#### Scenario: Original metadata type comparison
- **WHEN** BSL compares the type of a nested metadata object with Type("ОбъектМетаданных") or Type("MetadataObject")
- **THEN** both comparisons SHALL be true
- **AND** both names SHALL resolve to the same type as the existing generic metadata lookup name

#### Scenario: Separate configuration root
- **WHEN** BSL compares the configuration root metadata type with Type("ОбъектМетаданных")
- **THEN** the types SHALL differ

### Requirement: Enumeration constant identity
The database adapter SHALL materialize supported enumeration constants using the declared enumeration reference type and existing enumeration member identity.

#### Scenario: Known member
- **WHEN** a constant contains a known enumeration member
- **THEN** its value equals that member obtained from the enumeration manager
- **AND** its type is the declared enumeration reference type

#### Scenario: Empty enumeration constant
- **WHEN** a constant contains the nil enumeration reference
- **THEN** its value equals the enumeration manager's empty reference

### Requirement: Metadata lookup for supported configuration types
Metadata SHALL expose НайтиПоТипу/FindByType for supported acquired configuration reference, object and selection types and return their existing metadata owner identity.

#### Scenario: Acquired reference owner
- **WHEN** FindByType receives a supported acquired configuration reference, object or selection type
- **THEN** the returned metadata value equals that owner obtained from its metadata collection
- **AND** Russian and English method names agree

#### Scenario: Primitive type
- **WHEN** FindByType receives Number
- **THEN** it returns Undefined

#### Scenario: Invalid argument
- **WHEN** FindByType receives a number instead of a Type value
- **THEN** it raises a type error

#### Scenario: Unsupported owned type family
- **WHEN** a configuration-owned type has no supported owner association
- **THEN** lookup raises an explicit unsupported error instead of reporting absent metadata

### Requirement: Chart predefined account references
An acquired chart-of-accounts manager SHALL expose its declared predefined account names as read-only properties and as bare properties in its own manager module. Local and module variables SHALL retain their existing precedence. Resolution SHALL return the actual record reference through the current session adapter, never a fabricated reference derived from the predefined metadata GUID.

#### Scenario: Bare and qualified account identity
- **WHEN** the owning manager reads a declared predefined account both bare and through ПланыСчетов or ChartsOfAccounts
- **THEN** both reads and the corresponding query VALUE expression return equal chart references

#### Scenario: Qualified reference type
- **WHEN** the reference type is looked up as ПланСчетовСсылка or ChartOfAccountsRef qualified by its acquired owner name
- **THEN** both names resolve to the type returned by TypeOf for that account reference

#### Scenario: Read-only property
- **WHEN** code assigns a predefined account property on the manager
- **THEN** the existing read-only property error is returned

#### Scenario: Existing variable precedence and scope
- **WHEN** a local or module variable shadows a predefined account name
- **THEN** the variable wins
- **AND** an unrelated module does not acquire the bare account name

#### Scenario: Unresolved account
- **WHEN** the session adapter cannot resolve the declared account record
- **THEN** the existing explicit missing-record or adapter error is preserved

### Requirement: Chart empty reference receiver method
A chart-of-accounts manager SHALL provide ПустаяСсылка and EmptyRef with zero arguments, both qualified and as declared receiver methods in its own manager module. The result SHALL have the owner's chart reference type and nil UUID, compare equal across aliases, and report empty through Пустая and IsEmpty without a database read.

#### Scenario: Empty reference in manager context
- **WHEN** the owning chart manager calls either alias bare or qualified
- **THEN** the results are equal empty references of that chart's declared type
- **AND** they differ from its filled predefined account reference
