# Design

Reuse EnumKind, EnumValue and their existing member/name/presentation tables. Add exactly the documented members Активный / Active, Пассивный / Passive, АктивноПассивный / ActivePassive. Do not add another resolver path or ORM implementation of a system enum.

Vendor syntax helper 8.3.27.2342: platform-syntax-help:syntax-context:objects/catalog2/catalog1451/AccountType.html and its three property documents Active2137.html, Passive2138.html and ActivePassive2139.html. A dedicated native script measure-account-type.bsl probes presentations, concrete type and English aliases via catchable dynamic expressions in a throwaway file infobase. Retain the platform output as the test oracle, never generate it from open-bsl. Implementation waits for the completed measurement and current workspace gate.
