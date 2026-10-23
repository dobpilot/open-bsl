# Design

Native evidence is measure-daylight-time-offset.bsl and its 22-line platform output, recorded on 1C 8.3.27.2342 in a separate file infobase. Syntax help confirms two optional parameters and both names.

Reuse TimeZone::offset_seconds and standard_offset_seconds, and their named variants. Compute the difference for the same UTC instant, reusing the existing standard-offset argument conversion. No second TZif parser, external timezone dependency or hardcoded regional rules. Explicit named zones retain existing invalid-zone failure. Omitted date uses host clock.

The native oracle confirms GMT offsets produce zero, New York and Berlin produce zero in January 2024 and 3600 in July 2024. Valid date strings are accepted; invalid date strings, numeric and array dates fail. These are measured conversions, not deductions from help. Default-zone observations reflect this host and must not imply all hosts have zero offset.

This function does not implement all of UNF mail objects or claim successful session startup. Repeat original startup and record the next actual result.

The interpreter replay matched all 22 native observations after explicitly removing numeric grouping nonbreaking spaces and mapping the Russian Number type presentation to the native English presentation. Raw outputs differ in locale presentation; they are not byte-identical. Native oracle bytes were retained unchanged. Focused new daylight tests and all four existing standard-offset tests passed.
