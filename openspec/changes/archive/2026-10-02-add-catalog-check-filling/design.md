# Design

Use the existing catalog object protocol and configuration-instance handler dispatch. Do not add a parallel interpreter or alter name resolution.

The native probe on 1C 8.3.27.2342 returns ERROR from the object-module body, FALSE for an empty object, FALSE with Code alone, TRUE with Description alone, TRUE after both are assigned, TRUE through the English alias and ERROR for one argument. Its seven-line receipt is measure-catalog-check-filling.platform.txt.

These observations establish that Code is not mandatory in this fixture. Additional preserved handler, configured-attribute, scalar and common-field probes establish the implemented cases. SDK 0.9.6 now exposes field settings and complete ordered standard tables from measured Config/XML pairs. Missing settings remain distinct from a known absence of checks. Original Countries has ShowError on both Code and Description; the minimal fixture's defaults are never substituted for other catalogs.

User-message delivery and the filling-check event are part of the real operation, not optional success-path substitutes. Reuse existing host message and instance services where applicable.

The twenty-four-line handler probe supplies FALSE and a one-element array containing Description. Clearing the array returns TRUE; clearing it and setting refusal returns FALSE. Mutating it to Code or Description returns FALSE for empty fields, while Код, Наименование and the measured unknown path return TRUE. Lowercase description and uppercase DESCRIPTION also return FALSE. Assigning a new empty array to the parameter returns FALSE, unlike clearing the original array. Standard validation must retain these measured path and mutation semantics. A populated Description probe must distinguish replacement with a new Code array; custom-field paths need further probes.

Three additional observations settle replacement: with populated Description and empty Code, mutating the original array to Code returns FALSE, while assigning a new empty or Code array returns TRUE. Retain the original array identity for post-handler validation; do not use the reassigned second parameter. The complete handler receipt now contains twenty-seven lines.

Supported own values are single string, number, Boolean and date types; common values currently support single strings. Existing runtime type adjustment is reused. Unsupported unions, references and tabular sections produce explicit errors. Other known standard paths are explicitly unsupported rather than treated as the measured unknown path.

Native ordering is standard fields, own fields, then applied common fields. Messages reuse the existing host sink or stdout route. System language selects the measured English/Russian template; the existing current configuration language selects a custom-field synonym. Native en configuration with ru system language verifies their independence. Other template languages remain explicitly unsupported.
