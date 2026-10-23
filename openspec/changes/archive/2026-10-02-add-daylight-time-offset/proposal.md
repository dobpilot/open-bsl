# Add daylight time offset

## Why
Original UNF Document.Событие.ObjectModule calls DaylightTimeOffset while converting mail timestamps. The function is absent from the builtin table.

## What Changes
Expose СмещениеЛетнегоВремени / DaylightTimeOffset with zero through two arguments. Reuse existing host time-zone services and TZif decoding. Preserve the measured native input conversions and error cases.

## Impact
Affects bsl-rt builtin dispatch, generated reference and focused tests. No new dependencies or changes to existing timezone APIs.
