## ADDED Requirements

### Requirement: Reached StrFind argument conversion
StrFind SHALL convert measured simple text and needle arguments through the existing formatter. Numbers SHALL use no grouping and a decimal point. Unsupported argument types SHALL retain the previous strict execution path. Native Undefined, number, boolean and Null controls SHALL replay. An empty needle SHALL return one, including empty text and the retained direction, start and occurrence controls. Existing measured non-empty searches SHALL retain their results.

#### Scenario: Original parameter substitution
- **WHEN** the original configuration passes Undefined text and a non-empty needle
- **THEN** StrFind returns zero instead of a string type error

#### Scenario: Native empty needle
- **WHEN** the converted needle is empty
- **THEN** StrFind returns one as measured on 1C
