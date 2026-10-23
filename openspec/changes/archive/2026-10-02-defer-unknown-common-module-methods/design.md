# Evidence

`measure-module-unknown-common-method.platform.txt` records 42, ERROR, ERROR, 42.
The parent common module has a valid export and an export calling a method
absent from a known child's export surface. A procedure repeats the same case.
Neither body marker nor the argument marker appears. The failure is catchable, and the
valid neighboring export remains available afterwards.

# Mechanism

Use the existing opt-in `defer_common_module_name_errors`, enabled by Session.
Record an unresolved imported call internally while resolving the function
body. Only that error class becomes a catchable Raise at function entry,
using the existing mechanism for deferred method arity errors. Do not change
the deferred whole-module handling of unknown variables.

An open object-method fallback would evaluate arguments before discovering
the missing method, contradicting the native receipt. No new VM instruction,
bytecode version, dependency or public error variant is necessary.

Known exports keep normal linking and arity validation. Local/module variable
priority, unknown bare-function diagnostics, managers, object modules and
standalone code remain unchanged. The limited policy does not claim unmeasured
default-parameter or initializer behavior.

The buh payroll module has no source text in the database or native XML
export. Deferral permits an unused failing neighboring export to coexist with
valid code; actual execution still fails explicitly and is not a substitute
for implementing a protected module.
