# Design

Parse the option with existing database modifiers and pass the selected kind to
SessionFactory. Reuse the existing SessionKind enum, State, preprocessing,
startup handler, and database adapter. Do not add a second execution path.

Only foreground kinds are selectable here. A real background session requires
a parent, inherited parameters, and its own connection; labelling an ordinary
CLI session as a background job would not satisfy that contract. Background
verification remains a separate parent-bound scenario in the main plan.

Modifiers after the script path remain script arguments. Invalid values and
duplicate options fail before connecting to the database. Without config-db
the modifier is invalid, while ordinary commands retain their behavior.
