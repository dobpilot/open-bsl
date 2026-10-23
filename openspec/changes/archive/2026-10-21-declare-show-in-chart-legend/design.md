# Design

Use the existing EnumKind, EnumValue and TypeId tables; preserve current resolution and variable precedence. Native control in target/diagnostics/show-in-chart-legend-native-1016 proves aliases, type names, presentations, iteration order and unsupported primitive ordering. The complete measured enumeration is small: Auto, Show, DontShow in that order. Vendor syntax help confirms the names. This declaration does not implement chart rendering.

Native iteration order is retained as measurement evidence and table order. Enumeration-container iteration is not added by this declaration; the existing runtime rejects it with NotIndexable. Focused facade checks therefore inspect the three members directly.

The existing enumeration-container type/presentation convention also differs from the native Enum-prefixed name. This change does not modify that shared convention; only member aliases, types and presentations are contractual here. Native container observations remain in the oracle.
