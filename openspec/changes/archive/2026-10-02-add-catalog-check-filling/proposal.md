# Add catalog object filling checks

## Why
Original buh reaches CheckFilling in the Countries object module. A successful placeholder would bypass configuration validation and cannot execute the original contract.

## What Changes
Implement the zero-argument catalog object CheckFilling method using acquired filling-check settings and the existing object-instance event dispatch. Preserve native initialization rejection and acquire sufficient settings before enabling standard validation.

## Impact
ORM catalog objects, configuration acquisition, session tests and generated API reference. No new core or VM dependencies.
