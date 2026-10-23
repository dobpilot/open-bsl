## ADDED Requirements

### Requirement: Configuration compilation errors identify their source
When bytecode generation rejects a configuration method or module initializer,
the diagnostic SHALL identify the configuration module and, for a method,
its function or procedure name. Standalone compilation SHALL keep its
existing diagnostic.

#### Scenario: A configuration method exceeds the frame limit
- **WHEN** a method in a named configuration module uses more local slots than the bytecode frame can represent
- **THEN** compilation fails with the existing limit error and names both the module and method

#### Scenario: A standalone script exceeds the frame limit
- **WHEN** a standalone script uses more local slots than the bytecode frame can represent
- **THEN** compilation fails with the existing standalone limit error

### Requirement: Sparse local names fit the bytecode frame
A configuration method without exception handlers, dynamic source evaluation,
or debug-local materialization, whose source declares more than 255 distinct
local names SHALL compile when its simultaneously needed local values and
temporary registers fit the existing bytecode frame. The compiler SHALL preserve parameter
positions, module variable slots, by-reference updates, and observable
behavior. A method that cannot fit SHALL fail with the existing frame-limit
error; bytecode register fields SHALL remain unchanged.

#### Scenario: Configuration method has many sequential locals
- **WHEN** a configuration method has more than 255 distinct locals that are used in non-overlapping phases
- **THEN** it compiles and yields the same result as its source semantics

#### Scenario: Values do not fit simultaneously
- **WHEN** more local values must coexist than the frame can represent
- **THEN** compilation fails explicitly without truncating register indices

### Requirement: Configuration method arity failures are lazy
When a configuration module contains a method whose body calls a local or imported function with too many arguments or omits a required trailing argument, engine construction SHALL retain the valid methods of that module. Calling the invalid method SHALL raise a catchable arity error. Strict arity validation for standalone scripts and compiled entries SHALL remain unchanged.

#### Scenario: Another export in the same module is valid
- **WHEN** a configuration module has one valid export and another export with a too-many-arguments call
- **THEN** engine construction succeeds and the valid export executes normally

#### Scenario: Invalid local or imported call is executed
- **WHEN** the invalid export calls a local or imported function with too many arguments or omits a required trailing argument
- **THEN** that export raises a catchable arity error when invoked

#### Scenario: Standalone entry remains strict
- **WHEN** a compiled entry calls an imported function with too many arguments
- **THEN** compilation returns the existing arity diagnostic

### Requirement: Common-module names resolve to module values
A configuration module SHALL resolve an unshadowed bare imported common-module name to the same Session module-reference path used by dynamic evaluation. Local and module variables SHALL retain priority. The reference SHALL preserve the existing initialization behavior of its target; an invalid target SHALL fail on access without being mistaken for an invalid variable name in the reader. No new interpreter or rewritten configuration source SHALL be introduced.

#### Scenario: Static arguments and assignment
- **WHEN** a configuration method passes its own or another common-module name as an argument, or assigns that name to a variable
- **THEN** it obtains the corresponding module reference, as measured by the native module-argument-context control

#### Scenario: Dynamic common-module reference
- **WHEN** a configuration method evaluates a string containing an imported common-module name and calls an export on the resulting value
- **THEN** the call reaches that Session's module instance, including its initialized module variables

#### Scenario: Configuration module reference type
- **WHEN** a configuration method reads an imported common-module name
- **THEN** the resulting reference has type `ОбщийМодуль` / `CommonModule`, and standalone module references retain their existing `МодульBSL` type

#### Scenario: Invalid target stays invalid
- **WHEN** reading a common-module reference initializes a target with a genuine module compilation error
- **THEN** its original module initialization failure remains explicit

#### Scenario: Ordinary unknown variable
- **WHEN** a module reads a name that is neither a variable nor an imported module alias
- **THEN** the existing semantic diagnostic and configured deferral policy remain unchanged

### Requirement: Named modules resolve self-qualified calls
A named or global configuration module SHALL resolve its own module name
through the same module-reference path by which another module resolves it.
The self-reference SHALL NOT add an initialization-graph cycle. Reading the
bare self name SHALL return that same module reference, including when
passed as an argument or assigned to a variable.

#### Scenario: Self-qualified exported call
- **WHEN** a named module calls one of its exports through its own module name
- **THEN** the call executes on that module's existing instance

#### Scenario: Bare self name
- **WHEN** a named module reads its own unshadowed bare module name as a value
- **THEN** it obtains its own module reference without an initialization cycle

### Requirement: Unreachable branch targets preserve scoped source blocks
For the retained overlapping-method controls, source recovery SHALL preserve
all declarations and callable behavior when a command unreachable from every
entry targets the middle of an otherwise valid expression. Such a target
SHALL NOT split the current method's expression block. Recovery SHALL use the
active method's labels and SHALL retain explicit rejection of unsupported
reachable instructions or invalid reachable targets.

#### Scenario: Unreachable jump targets a public initialization expression
- **WHEN** the scoped-foreach expression-target control is recovered into BSL
- **THEN** both nonempty and empty public loops and the private wrapper replay all seven native receipt lines
- **AND** all three method declarations remain present

#### Scenario: The same jump becomes reachable
- **WHEN** a mutation makes the unsupported mid-expression jump reachable from a method entry
- **THEN** recovery rejects the image instead of silently omitting that path

### Requirement: Scoped conditional returns preserve the shared result
For the measured overlapping-method return-join profiles, recovery SHALL
preserve both alternatives of a conditional expression feeding one shared
return command. The resulting source SHALL compile and execute through the
existing BSL front end and VM, retain all declarations, and preserve private
caller arguments. Unsupported merge states or cleanup paths SHALL remain
explicit recovery errors.

#### Scenario: False-branch return join
- **WHEN** the scoped-return-join-40 control is recovered
- **THEN** both public alternatives and the private wrapper replay every native receipt line

#### Scenario: True-branch return join
- **WHEN** the scoped-return-join-41 control is recovered
- **THEN** branch polarity and both returned types remain identical to the native receipt

#### Scenario: Unsupported return cleanup
- **WHEN** a mutation adds an unsupported operation to the shared return's cleanup path
- **THEN** recovery rejects the image instead of omitting that operation

### Requirement: Infeasible conditional edges preserve call preparation
For measured scoped literal-condition profiles, a conditional edge proven
infeasible by the existing reachability analysis SHALL NOT split argument
preparation from its call. Recovery SHALL preserve the call, result and
caller arguments. A feasible or unknown unsupported edge into an incomplete
call preparation SHALL remain an explicit error.

#### Scenario: Literal True before a false jump
- **WHEN** scoped-infeasible-call-edge-40 is recovered
- **THEN** its complete native public and private observations are reproduced

#### Scenario: Literal False before a true jump
- **WHEN** scoped-infeasible-call-edge-41 is recovered
- **THEN** its complete native observations are reproduced with unchanged branch polarity

#### Scenario: The incomplete call edge can execute
- **WHEN** a mutation makes the mid-call conditional edge feasible or unknown
- **THEN** recovery rejects that unsupported path rather than dropping the call

### Requirement: Scoped collection loops preserve outside back-edge trampolines
For the measured scoped foreach profiles with an outside direct-jump
trampoline, recovery SHALL preserve iteration, conditional continuation,
break and iterator cleanup through the existing BSL compiler and VM. All
method declarations and caller arguments SHALL be retained. Unsupported
trampoline operations, cycles or destinations SHALL remain explicit errors.

#### Scenario: Back-edge outside the loop body
- **WHEN** scoped-foreach-outside-trampoline is recovered
- **THEN** its entire native receipt is reproduced for nonempty and empty collections and private caller observations

#### Scenario: Conditional continuation and early break
- **WHEN** scoped-foreach-outside-conditional is recovered
- **THEN** false comparisons continue through the outside trampoline and a true comparison breaks with the same native result

#### Scenario: Unsupported trampoline
- **WHEN** the outside trampoline contains an operation other than the measured direct jump or does not lead to the iterator check
- **THEN** recovery rejects the image without omitting the operation

### Requirement: Scoped backward boolean assignments preserve reference targets
For the measured scoped ordinary procedure profile, recovery SHALL preserve
both alternatives of a backward boolean assignment and its reference target,
retain every method declaration, and execute the resulting source through
the existing BSL compiler and VM. Unsupported shared-entry mutations SHALL
remain explicit recovery errors.

#### Scenario: Both caller-visible boolean alternatives
- **WHEN** scoped-backward-bool is recovered
- **THEN** the exported procedure reproduces the true and false reference-parameter observations and every private caller receipt line

#### Scenario: An extra entry reaches the assignment join
- **WHEN** a mutation introduces another entry into the protected boolean join
- **THEN** recovery rejects the unsupported shared stack state

### Requirement: Measured private zero-label tables preserve callable routes
For the measured four-slot private procedure profile with 27 zero-address
labels, recovery SHALL retain every declaration, preserve private and public
routes, message ordering, return values and caller arguments, and execute
through the existing BSL compiler and VM. Unsupported label targets or frame
mutations SHALL remain explicit recovery errors.

#### Scenario: Private table has 27 zero labels
- **WHEN** zero-label-procedure-27 is recovered
- **THEN** every native public/private message and value observation is reproduced

#### Scenario: Private table contains an unmeasured address
- **WHEN** a mutation changes a private zero label to an unsupported address
- **THEN** recovery rejects the unsupported route instead of choosing another frame's labels

### Requirement: Measured private string frames preserve route observations
For the measured four-slot flags2 procedure with27 zero labels and a string
constant in slot0, recovery SHALL preserve all declarations, route messages,
return values and unchanged caller arguments through the existing BSL
compiler and VM. This includes the measured declared Var size18 with four
actual descriptors. Unmeasured frame or constant combinations SHALL remain
explicit recovery errors.

#### Scenario: String alias replaces the numeric constant
- **WHEN** zero-label-procedure-27-string is recovered
- **THEN** every native public/private route and caller observation is reproduced

#### Scenario: Declared frame size exceeds descriptor count
- **WHEN** zero-label-procedure-27-string-frame18 is recovered
- **THEN** all complete native receipt lines remain identical to the measured string-frame control

### Requirement: Ordinary scoped bodies reuse complete module declarations
For the measured ordinary shared-body module profile, recovery SHALL preserve
ordinary method signatures, reference parameters, defaults, visibility and
external/global bindings while recovering each body in its own frame and
label context. The source SHALL execute through the existing BSL compiler
and VM. The complete image SHALL remain rejected when another declaration
has an unsupported header or body; no successful partial source SHALL be
returned.

#### Scenario: Ordinary methods share a body with contextual declarations
- **WHEN** ordinary-shared-context is recovered
- **THEN** both ordinary declarations remain, the global and external declarations retain existing resolver bindings, and every native receipt line is reproduced

#### Scenario: An unsupported private declaration remains
- **WHEN** a mutation appends an unsupported private declaration to the same module
- **THEN** recovery rejects the entire image instead of returning only the ordinary methods
