# value-list-unload Specification

## Purpose
Define ordered ValueList value extraction into a fresh independent array without modifying the source list.

## Requirements

### Requirement: Unload value list values into an independent array
ValueList SHALL expose ВыгрузитьЗначения and UnloadValues through existing component dispatch with no arguments. Each call SHALL return a new array containing item values in current list order, without replacing values by presentations or item objects. The call SHALL NOT modify the list.

#### Scenario: Empty list
- **WHEN** either method name is called on an empty list
- **THEN** it returns an empty Array

#### Scenario: Ordered values
- **GIVEN** items with string, number and Undefined values and distinct presentations
- **WHEN** values are unloaded
- **THEN** the array contains those values in list order

#### Scenario: Independent slots
- **WHEN** a slot in the returned array is replaced
- **THEN** the corresponding item value in the original list remains unchanged

#### Scenario: Excess arguments
- **WHEN** an argument is supplied
- **THEN** the existing component arity validation rejects the call

#### Scenario: Mutable values retain identity
- **GIVEN** an item value is a mutable Structure
- **WHEN** a property of that structure is changed through the returned array
- **THEN** the item value observes the same property change
