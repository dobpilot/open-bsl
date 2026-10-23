# Design

Use the existing EnumKind, EnumValue and exhaustive name/display tables. ChartLabelLocation already exposes EdgeAuto; add the subsequently reached EdgeInside to that same kind. Reuse the existing type aliases and value-to-type mapping. ChartScaleLabelLocation.None remains unchanged. No new type, component, dependency, renderer or resolver mechanism is needed.

Native 1C 8.3.27.2342 controls confirm the reached aliases, both type lookup names and English-session presentations: EdgeAuto=`Edge auto`, EdgeInside=`Edge inside`, ChartScaleLabelLocation.None=`Do not show`. EdgeInside differs from EdgeAuto; `InsideEdge` is not a valid alias, and primitive less-than ordering of the two label values fails. Evidence: `docs/research/fixtures/chart-label-location-native-1015/`, `chart-scale-label-location-native-1015/` and `chart-label-inside-1016/`.

The earlier original manager compilation passed EdgeAuto and reached the scale label. The latest original BUH replay passes the declared Chart constructor and reaches EdgeInside in the same classification manager. This extends the existing active label-location change, rather than creating a separate change for another member of the same enum. No archived contract is rewritten. Full chart behavior and other unneeded enum members remain outside this bounded declaration step.
