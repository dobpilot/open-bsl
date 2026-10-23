# object-module-properties Specification

## Purpose
Enable original configuration object modules to access their receiver properties through the facade while preserving variable priority and measured object initialization behavior.

## Requirements

### Requirement: Finite receiver property context
The facade SHALL allow the host to declare a finite property context for a
configuration module. An unqualified name in that context SHALL read or assign
the current receiver property through existing property operations. Modules
without this context SHALL retain their previous resolution behavior.

#### Scenario: Unqualified property
- **WHEN** an object module with a declared Code property reads or assigns Code
- **THEN** the operation uses the current object instance
- **AND** assignment does not introduce an implicit local variable

#### Scenario: Explicit variable priority
- **WHEN** a parameter or explicit local or module variable has the property name
- **THEN** it retains priority over the receiver property

#### Scenario: Unknown identifier
- **WHEN** an undeclared name is absent from the receiver property table
- **THEN** existing undefined-name and implicit-local rules apply

### Requirement: Context continuity
Dynamic execution and worker reconstruction SHALL retain declared receiver
property contexts consistently with their compiled modules.

#### Scenario: Dynamic property access
- **WHEN** dynamic code reads or assigns a supported receiver property
- **THEN** it affects the same receiver as an ordinary method of that module

### Requirement: Object exchange and additional properties
New catalog objects SHALL provide the measured additional-property collection
and data exchange settings. These mutable values SHALL belong to the concrete
object and SHALL NOT perform database writes by themselves.

#### Scenario: Independent settings
- **WHEN** a caller changes supported settings or additional properties of one object
- **THEN** another newly created object retains its own defaults
- **AND** the operation does not write a database row

### Requirement: Object selection after body initialization
A newly created catalog object SHALL become selected after its object module
body finishes. Supported Code, Description and Ref reads SHALL be available in
the body, but Code/Description assignment and IsNew SHALL fail before selection.

#### Scenario: Initialization before selection
- **WHEN** the object module body reads standard attributes and attempts supported writes or IsNew
- **THEN** reads succeed and the writes and IsNew fail
- **AND** the same operations succeed after creation completes, subject to acquired qualifiers

### Requirement: Owned finite receiver property names
The facade SHALL accept a finite receiver property-name context owned by the engine for a configuration module. Names SHALL use existing case-insensitive comparison and variable priority. Dynamic code and reconstructed background workers SHALL retain this context. Existing descriptor-table registration SHALL remain supported.

#### Scenario: Configuration-owned name
- **WHEN** the host declares an acquired catalog attribute name for its object module
- **THEN** an unqualified read or assignment resolves through the current object receiver
- **AND** a local, parameter or module variable with the same name retains priority

#### Scenario: Unknown name outside the context
- **WHEN** an identifier is absent from the module's finite property context and existing name scopes
- **THEN** the existing undefined-variable diagnostic remains unchanged

### Requirement: Acquired attribute declarations do not fabricate values
The ORM SHALL retain catalog attribute names from authoritative acquired metadata without including unrelated nested fields. An attribute declared for compilation but lacking its typed runtime implementation SHALL fail explicitly on reads and writes, naming that attribute.

#### Scenario: Declared but unavailable attribute
- **WHEN** original object code reads or writes a declared attribute without a real value implementation
- **THEN** execution raises an explicit unavailable-attribute error
- **AND** no Undefined default or successful untyped assignment is fabricated

### Requirement: Acquired catalog tabular section receiver names
The configuration host SHALL retain authoritative owned catalog tabular section names and include them in the object's finite receiver property context. Nested row attributes SHALL NOT become direct object receiver properties. Unknown acquisition SHALL remain distinct from a known empty list.

#### Scenario: Native section declaration
- **WHEN** a catalog contains an acquired section Состав with a nested Пользователь attribute
- **THEN** bare Состав resolves through the catalog object receiver
- **AND** Пользователь does not become a direct receiver property
- **AND** existing parameter, local and module variable priority remains unchanged

### Requirement: Unavailable section values fail explicitly
A declared catalog section lacking its typed runtime implementation SHALL raise an explicit named error on access rather than fabricate an empty collection or accept an untyped assignment.

#### Scenario: Declared section accessed before implementation
- **WHEN** code reads or writes a declared section whose runtime value is unavailable
- **THEN** execution fails explicitly naming that section
- **AND** no substitute section value or database write is produced

### Requirement: New catalog object staged reference
A catalog object SHALL expose ПолучитьСсылкуНового / GetNewObjectRef with zero arguments and УстановитьСсылкуНового / SetNewObjectRef with one argument. These methods SHALL use the existing owning object receiver and preserve the measured native default, validation and preselection behavior. Staging SHALL NOT write a database record or manufacture another object's reference type.

#### Scenario: Original object module stages a reference
- **WHEN** an original catalog object module stages an own reference and reads it again
- **THEN** getter results retain its owning reference type and UUID according to the preserved native oracle
- **AND** the source runs through existing finite receiver contexts without preprocessing

#### Scenario: Native argument and selection rules
- **WHEN** a caller passes an invalid value or accesses the methods during object-module initialization
- **THEN** behavior matches the preserved native measurement
- **AND** invalid arity is rejected through the existing method-table contract

### Requirement: Staged catalog references belong to their object
Staged-reference state SHALL belong to the explicit catalog-object instance. Staging SHALL remain separate from record persistence and SHALL NOT be copied into unrelated new objects or sessions by shared mutable state.

#### Scenario: Two new objects
- **WHEN** one new object stages an own reference
- **THEN** another new object retains its measured initial staged-reference state
- **AND** no database connection or session rights are transferred with the reference

### Requirement: New catalog item folder flag
The already supported new catalog item object SHALL expose read-only ЭтоГруппа / IsFolder as a boolean standard property through the existing receiver context. Its value and object-body availability SHALL match the preserved native oracle. Declaring the property SHALL NOT fabricate persisted records or implement folder creation implicitly.

#### Scenario: Original object module reads the flag
- **WHEN** an original catalog object module reads the flag bare or through the object
- **THEN** it obtains the measured flag of that specific new item
- **AND** Russian and English aliases agree without changing ordinary variable priority

#### Scenario: Assignment is rejected
- **WHEN** code attempts to assign the folder flag
- **THEN** it receives a read-only-property error
- **AND** the object flag remains unchanged

### Requirement: New catalog item deletion mark
A new catalog item SHALL expose ПометкаУдаления / DeletionMark through the existing receiver property context. Its default, object-body availability, assignment conversion and failure behavior SHALL match the preserved native oracle. The property SHALL belong to the concrete object and SHALL NOT implicitly write a database record.

#### Scenario: Original module reads and assigns the property
- **WHEN** an original object module reads or assigns the deletion mark
- **THEN** the existing engine uses that object's measured state and conversion rules
- **AND** Russian and English aliases address the same property

#### Scenario: Independent new objects
- **WHEN** one new object's deletion mark changes
- **THEN** another new object retains its measured initial value
- **AND** ordinary local and module variable priority remains unchanged

### Requirement: Reusable Boolean conversion through the facade
The public open-bsl facade SHALL expose convert_to_boolean taking a borrowed Value and returning RtResult<bool>. It SHALL preserve the existing Boolean builtin conversion and errors. The deletion-mark setter SHALL default conversion errors to false without changing the builtin or table-column behavior.

#### Scenario: Existing builtin behavior remains unchanged
- **WHEN** a host calls the helper with a recognized Boolean value, number or string
- **THEN** it receives the same Boolean result as the existing Boolean builtin
- **AND** unsupported inputs return the existing conversion error

#### Scenario: Consumer uses only the facade
- **WHEN** an external consumer invokes the conversion helper
- **THEN** it needs no internal runtime crate name

### Requirement: Declared catalog object write boundary
The existing catalog object receiver SHALL declare Записать / Write with zero arguments. Original modules containing this method SHALL compile through the existing receiver method mechanism. A previously callable exported object-module procedure named Записать / Write SHALL retain its behavior; this addition SHALL NOT shadow it. Until persistence is connected, invocation SHALL fail explicitly without successful no-op behavior, database writes or a fabricated persisted reference.

#### Scenario: Uncalled procedure contains Write
- **WHEN** an original object module declares an exported procedure containing bare Записать()
- **THEN** the existing interpreter can compile the declared procedure
- **AND** ordinary local and module name priority remains unchanged

#### Scenario: Persistence is invoked before integration
- **WHEN** configuration code invokes Записать or Write on a catalog object
- **THEN** it receives an explicit persistence-not-connected failure
- **AND** the object's persisted reference and new-object status remain unchanged

#### Scenario: Wrong arity
- **WHEN** the method receives an argument
- **THEN** the existing method arity checks reject the call

### Requirement: Read-only configuration export inspection
CallContext SHALL expose configuration_has_export(module, name) returning RtResult<bool> through the existing executor service. It SHALL inspect compiled exported names using case-insensitive comparison without executing a module body or procedure. Missing executor or unavailable module information SHALL fail explicitly.

#### Scenario: Existing exported Write remains callable
- **WHEN** an object module exports a procedure named Записать
- **THEN** adding the platform Write declaration does not intercept that procedure
- **AND** the component can inspect export presence without executing it

#### Scenario: Missing or nonexported name
- **WHEN** a compiled module has no exported procedure with the inspected name
- **THEN** inspection returns false without running the module

### Requirement: Staged catalog predefined data name
A concrete catalog object SHALL expose ИмяПредопределенныхДанных / PredefinedDataName through the existing receiver context. Initial state, object-body access, valid and invalid assignments SHALL match the preserved native oracle and use acquired predefined names. Staging this property SHALL NOT imply database persistence.

#### Scenario: Original module reads the standard property
- **WHEN** an original catalog object module reads the predefined data name
- **THEN** the existing receiver returns the native-measured staged value
- **AND** ordinary local and module variable priority remains unchanged

#### Scenario: Invalid assignment preserves state
- **WHEN** an invalid predefined data name is assigned
- **THEN** the receiver follows the native-measured failure and retained-value behavior

#### Scenario: Concrete object isolation
- **WHEN** one object's predefined data name changes
- **THEN** another object's value remains independent

### Requirement: Receiver property priority over imported aliases
A declared receiver property SHALL take priority over a same-named imported common-module alias in reads and qualified method calls. Parameters and explicit local or module variables SHALL retain priority over that property. Modules without receiver property context SHALL retain existing imported-module resolution.

#### Scenario: Colliding property method
- **WHEN** an object module calls Количество on its ДополнительныеСвойства property and a same-named imported module exports Количество returning 999
- **THEN** the call SHALL use the receiver property and match explicit ЭтотОбъект access

#### Scenario: Parameter shadows property
- **WHEN** the object method receives a same-named parameter containing a one-element structure
- **THEN** calling Количество SHALL return 1 from the parameter

#### Scenario: No receiver context
- **WHEN** the same alias is used in a module without a colliding receiver property
- **THEN** the imported module export SHALL remain callable

### Requirement: New catalog object predefined flag
The supported new catalog object SHALL expose read-only Предопределенный / Predefined through its receiver context. Both aliases SHALL return Boolean false initially and during the object body, true when the predefined data name is nonempty, and false after clearing it. Assignment to the flag SHALL fail without changing the name or flag.

#### Scenario: Name transitions
- **WHEN** a new object's acquired valid predefined name is assigned and then cleared
- **THEN** both aliases SHALL report true and then false

#### Scenario: Body access
- **WHEN** the new object's body reads bare Предопределенный
- **THEN** it SHALL read the current object's false initial flag

#### Scenario: Read-only flag
- **WHEN** code assigns the flag
- **THEN** assignment SHALL fail and retain the previous name and flag

### Requirement: Document object Write declaration
The document object receiver SHALL declare Записать / Write with zero through two arguments through the existing receiver context. Previously callable exported methods with either name SHALL retain their behavior. Until persistence is connected, invoking the platform method SHALL fail explicitly without database writes, handler invocation, allocation, or object-state changes.

#### Scenario: Uncalled write branch compiles
- **WHEN** an original document object module contains bare Записать() in an uncalled procedure
- **THEN** compiling and creating the object SHALL not fail because that name is missing

#### Scenario: Finite platform signature
- **WHEN** a document receiver calls either alias with zero, one or two arguments
- **THEN** the method SHALL be resolved and execution SHALL report the unsupported persistence limit
- **AND** three arguments SHALL be rejected by the finite method arity

#### Scenario: Existing exported Write
- **WHEN** a document object module exports a callable Write or Записать method
- **THEN** the new platform declaration SHALL not intercept that export

#### Scenario: Failure preserves object state
- **WHEN** the platform Write method fails because persistence is not connected
- **THEN** the object's reference, new-object status, attributes and tabular sections SHALL remain unchanged
