# Design

The original module was acquired through the existing database code loader.
Its exported file-reading helper constructs a spreadsheet for `mxl` or a
geographical schema for `geo`, then calls `Read`. Static module reachability
does not prove this helper executes during session initialization.

Use `bsl-spreadsheet`, which already owns platform document types, rather than
placing a document object in the ORM or changing core dependencies. The native
8.3.27.2342 probe confirms constructor acceptance and the English type alias.
Its measured Russian type display is `Поле географической схемы`.

Execution deliberately reports the missing object model. Do not fabricate an
empty topology or a successful file read. Registering this constructor is an
intermediate compilation extension; geographic-format execution remains open.
The user approved the mechanical constructor-density expectation update from one constructor to two. Both aliases and the distinct declared type are registered; the facade test verifies dormant compilation, arity and explicit execution failure.
