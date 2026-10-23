# Why
Original demo compilation reaches `New PlannerItem` in `РаботаСРабочимКалендаремСервер`. Native 1C 8.3.27.2342 accepts the constructor and provides independently measured values and assignment behavior.

# What Changes
Register the constructor and implement the measured mutable standalone value through the existing component protocol and facade. Preserve unsupported behavior explicitly rather than manufacturing planner operations. Reuse runtime fixed-map storage for dimension values.

# Impact
Affected crates: bsl-session and, if a facade factory is needed for existing fixed-map storage, bsl-rt and open-bsl. No new dependencies or alternate interpreter. Full original startup remains a required subsequent check.
