# Why
Original BUH compilation reaches DataCompositionSettingsItemState.Enabled in the stock shelf-life report manager after the earlier placement enums are resolved.

# What Changes
Declare the native settings-item state enum using the existing enum and type tables, preserving both aliases and measured presentations.

# Impact
System enum declarations only. No changes to composition settings objects, report execution or name-resolution precedence. No new dependencies.
