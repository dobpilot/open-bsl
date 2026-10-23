# Support reached metadata type lookup names

## Why
Original UNF session initialization reaches Type("ОбъектМетаданных"). Native 1C controls confirm both Russian and English lookup names for nested metadata values. The ORM currently registers only its existing compatibility name.

## What Changes
Add measured lookup aliases to the existing generic metadata value type. Preserve the existing name, presentation and separate configuration-root type.

## Impact
One existing type descriptor gains two aliases. No new dependencies, resolver changes or object implementation are required.
