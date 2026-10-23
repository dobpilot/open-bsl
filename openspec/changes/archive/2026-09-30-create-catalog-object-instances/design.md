# Design

Native 1C 8.3.27.2342 evidence is stored in measure-catalog-create-item,
measure-catalog-create-item-number and measure-catalog-object-instance oracles.
CreateItem takes zero arguments, returns the exact CatalogObject type, creates a
new object with an empty reference, and does not write a database row. The probe
with string code starts with empty code and description. The probe with numeric
code starts with zero; DescriptionLength zero keeps a readable empty description
but rejects assignment. Numeric code assignment of "7" yields 7, and an invalid
string resets it to zero. These observations must not be generalized to fields
whose qualifier or conversion behavior has not been measured.

The SDK 0.9.5 ConfigDescriptor supplies CatalogStandardAttributes with code type,
code length and description length. The database source adapter converts this
into ORM-owned immutable metadata, without importing SDK types into bsl-orm.
An unknown shape remains absent. Creation without required settings fails
explicitly, rather than inventing a string code for every configuration.
DefaultPresentation is acquired independently: the native UNF corpus contains
both numeric and string codes with either presentation choice. The ORM therefore
retains the optional acquired choice and never derives it from code type or
description length. Presentation without this setting fails explicitly.
Object-module names are taken from the same ConfigurationCodeSnapshot that
already binds manager modules. Object and reference type identities are separate;
only immutable reference descriptors receive graph materializers.

Object modules use the same compiled programs, linking, dynamic compilation and
VM calls as configuration manager modules. Their variables must belong to the
concrete object. The native counters return 1, 1, 2 across first/second/first
objects, and ThisObject reports the corresponding object in both methods.
Common modules remain shared within a State and isolated across States.
The timing probe records the object's module initialization timestamp inside
the CreateItem call interval, then delays its first export call by 200 ms.
It confirms that creation initializes the body rather than postponing it until
the first method. Ordinary Сообщить in a separately copied object module is
not redirected by the form generator and is not evidence of initialization time.
The implementation mechanism for retaining object memory must avoid keeping a
live database connection in an engine descriptor or immutable worker recipe.
Mutable objects are not admitted to the immutable reference transfer codec.

The component owns a facade-exported ConfigurationInstance with an opaque
executor storage slot. The VM stores its existing ModuleState there, together
with the compiled program identity and a weak HostEnv ownership marker. The
runtime does not depend on VM types, and the slot never contains a database
connection. ObjectProtocol exposes this optional instance for ordinary component
callbacks. Active object memory is parked in that instance during a callback to
another object, then restored on return or BSL failure; this permits callbacks
back into a suspended object without cloning module variables. Common-module
memory continues to use the existing SessionModules swap. No opcode, module-slot
representation or global-name resolution rule changes.

Assignment measurements additionally show UTF-16 string truncation, canonical
number-to-string conversion, boolean conversion, numeric rounding and saturation.
Numeric catalog codes clamp negative input to zero. Existing table conversion
already implements the corresponding primitive conversions and qualifiers;
extract its existing adjustment into a facade-closed helper without changing
table behavior. Apply the separately measured nonnegative catalog-code bound in
the catalog object layer rather than changing signed numeric table columns.
The separate /Lru oracle records object type presentation as
`Справочник объект: ЗамерВременныхТаблиц`; its English counterpart is
`Catalog Object: ЗамерВременныхТаблиц`. Both qualified Russian and English
type lookup names designate the same object type, separately from CatalogRef.

Database writing and lifecycle callbacks are part of the broader data-access
work, not implicit side effects of CreateItem. Unsupported operations must fail
explicitly. The feature is incomplete until object-module isolation is tested;
recognizing the manager method alone does not satisfy this change.
