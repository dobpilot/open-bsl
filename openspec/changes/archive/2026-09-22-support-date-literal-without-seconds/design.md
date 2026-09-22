# Design

The source of the syntax is the installed 1C:Enterprise Syntax Assistant 8.3.27.2342, document `platform-syntax-help:platform-guides:def_Date`. It states that a date literal may omit all time fields or only seconds, that omitted seconds have value zero, and that every non-digit value inside the literal is ignored. Modules from the `unf` test information base independently demonstrate that both a twelve-digit literal and a date-only literal with hyphen separators occur in platform-accepted production source.

The lexer will collect only ASCII digits between the existing single quotes, ignoring every other character except the closing quote, and will admit the result only when it contains exactly 8, 12, or 14 digits. `BslDate::parse_digits`, which is shared by source literals and string date conversion, remains a digits-only boundary: it will parse hours and minutes for both time-bearing lengths and will supply zero seconds for the twelve-digit form. Calendar validation remains centralized in `BslDate::from_civil`.

The resulting components were measured on 1C:Enterprise 8.3.27.2342 in a `ru_RU` infobase with the Russian script variant. The literal `'202401151230'` produced year 2024, month 1, day 15, hour 12, minute 30, and second 0. A focused second measurement showed that `'2024аб01!!15'` produces 15 January 2024, confirming the documented general rule for non-digit separators. Every session anchor in the first measurement matched.

After both forms were implemented, the authorized `unf` compile gate passed every date literal and reached the next independent language barrier, `ExpectedContinuationBar`, in a multiline string. The loader still reported the same three unreadable `.7` resources; this change does not alter their handling.
