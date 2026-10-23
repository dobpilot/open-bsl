# Declare ShowInChartLegend

## Why
Original BUH session startup reaches a manager referencing ОтображениеВЛегендеДиаграммы.НеОтображать. The enumeration is missing.

## What Changes
Expose the three native measured values through the existing system enumeration tables, including Russian and English aliases and native presentations.

## Impact
Runtime public enumeration and type tables; facade regression tests and generated reference. No chart object model or dependency changes.
