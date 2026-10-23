# Declare the reached adjusted point constructor

## Why
Original BUH startup compilation reaches МоментВремениСУточнениемПериода inside the СверткаИнформационнойБазы manager. The ORM already declares the surrounding Boundary constructor with an explicit unsupported-execution error. The missing declaration blocks compilation before actual runtime requirements can be observed.

## What Changes
Declare the measured Russian and English constructor aliases, arity 1..3 and type aliases through the existing ORM library. Actual construction fails explicitly until its object model is implemented. Preserve native measurements for that later runtime requirement.

## Impact
ORM library descriptor and API reference; no core, VM, dependencies or source rewriting.
