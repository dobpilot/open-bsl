# configuration-module-images

## ADDED Requirements

### Requirement: Single executable body from a native module image
When a selected configuration module has no source text and has a supported
native executable image, the code loader SHALL recover an executable BSL body
and provide it through the existing configuration code snapshot. It SHALL
preserve procedure and function names, export visibility, parameters and
default values according to measured native format rules. Execution SHALL
use the existing open-bsl compiler and VM.

#### Scenario: Image-only common module
- **GIVEN** a supported native container containing an executable image and no source text
- **WHEN** the selected configuration snapshot is loaded
- **THEN** its module contains recovered declarations and executable bodies
- **AND** an exported call executes through the existing open-bsl pipeline

#### Scenario: Source text is available
- **GIVEN** a module container with source text and a native image
- **WHEN** the configuration snapshot is loaded
- **THEN** source text supplies the single effective module body
- **AND** the image does not create a second module instance

#### Scenario: Table header differs from the number of records
- **GIVEN** supported native tables with unsigned numeric headers that differ from their actual record counts
- **WHEN** the image is recovered and executed
- **THEN** all actual records are preserved and execution matches the measured native receipt
- **AND** malformed headers or record shapes are rejected
- **AND** numerical indices are checked against the actual records rather than fabricated padding

#### Scenario: Supported module initializer and private module storage
- **GIVEN** a supported native initializer and methods accessing private module variables
- **WHEN** a module instance is initialized and its exported methods are called
- **THEN** the initializer runs before the exported calls and its variables retain later method updates
- **AND** the initializer is not appended to the last method body
- **AND** independent object instances retain independent private module storage
- **AND** malformed initializer boundaries or unsupported commands reject the entire image

#### Scenario: Unused native label metadata
- **GIVEN** supported executable bodies with root or method label tables in the measured two-field form
- **WHEN** the image is recovered and executed
- **THEN** the label metadata does not alter the native results or parameter effects
- **AND** every label record and section header is validated
- **AND** duplicate label sections and malformed label records are rejected
- **AND** unsupported jump forms are still rejected

#### Scenario: SCSU-encoded native image
- **GIVEN** a supported image whose serialized tables use SCSU with its signature
- **WHEN** the code snapshot is loaded
- **THEN** declarations, constants and executable behavior match the equivalent UTF-8 image
- **AND** malformed or truncated encoding produces a module diagnostic without replacement characters or a partial body

### Requirement: Measured semantic preservation
Supported reconstructed bodies SHALL preserve the native behavior of value
and reference parameters, argument order, omitted arguments, expressions,
short-circuit operations, branches, loops, return and exception handling.
Compatibility evidence SHALL come from independently authored modules compiled
and executed on the real 1C platform.

#### Scenario: Native execution receipt
- **GIVEN** an independently authored source module, its native compiled image and its native execution receipt
- **WHEN** recovered BSL is executed through open-bsl with the same inputs
- **THEN** its result, parameter updates and observed side-effect order match the native receipt

#### Scenario: Supported continuation within collection iteration
- **GIVEN** a native collection loop with a supported conditional continuation
- **WHEN** its image is reconstructed and executed through the facade
- **THEN** continuation skips the remainder of that iteration
- **AND** collection traversal, parameter effects and the enclosing loop remain intact
- **AND** a jump outside the validated enclosing-loop boundary is rejected

#### Scenario: Conditional expression with a pending assignment target
- **GIVEN** a supported native conditional expression whose assignment target is already on the operand stack
- **WHEN** its image is reconstructed and executed
- **THEN** only the selected expression branch is evaluated
- **AND** the result is assigned to the original target
- **AND** malformed expression branches are rejected

#### Scenario: Nested conditional return
- **GIVEN** a supported nested conditional expression followed by a return
- **WHEN** the recovered body executes
- **THEN** its return value matches the selected native branch
- **AND** unselected nested branches are not evaluated
- **AND** shared forward joins do not turn expression branches into statement blocks

#### Scenario: Supported break within collection iteration
- **GIVEN** a native collection loop with a supported conditional break
- **WHEN** its image is reconstructed and executed through the facade
- **THEN** the break leaves the nearest loop
- **AND** effects before the break and the enclosing continuation remain intact
- **AND** a jump outside the validated iterator cleanup boundary is rejected

#### Scenario: Supported break within a while loop
- **GIVEN** a native while loop with a supported conditional break and a validated forward continuation
- **WHEN** its reconstructed body executes
- **THEN** the break leaves the nearest while loop without repeating its condition
- **AND** prior parameter effects and execution after the loop match the native receipt
- **AND** a jump to the immediately following instruction does not discard an instruction or value

#### Scenario: Multiple function returns with a shared exit
- **GIVEN** a supported function with conditional early returns and a final return
- **WHEN** its image is reconstructed and executed
- **THEN** the selected return value matches native execution
- **AND** skipped alternatives do not execute
- **AND** each return tail reaches the validated exit of that function
- **AND** an unknown command after a return is rejected rather than discarded

#### Scenario: Supported continuation within a while loop
- **GIVEN** a native while loop with a supported conditional continuation
- **WHEN** its reconstructed body executes
- **THEN** continuation skips the remaining statements of that iteration
- **AND** the loop condition is evaluated again before the next iteration
- **AND** parameter changes made before continuation remain visible
- **AND** a jump inside the condition instead of its validated beginning is rejected

#### Scenario: Counted-loop limit evaluated through a context method
- **GIVEN** a supported native counted loop whose upper bound invokes an object method
- **WHEN** the reconstructed body executes
- **THEN** the upper-bound call executes once before iteration
- **AND** changes to the object within the loop do not recompute the bound
- **AND** reference updates and the final loop counter match native execution

#### Scenario: Statement try block within a procedure
- **GIVEN** a supported native procedure with statements before and after a try block
- **WHEN** its recovered body executes with and without an error
- **THEN** the correct body or handler runs and execution continues after the block
- **AND** the handler can obtain the current error information snapshot
- **AND** parameter effects and the error description match native execution
- **AND** invalid handler, normal-exit or handler-end boundaries are rejected

#### Scenario: Omitted arguments of an external procedure
- **GIVEN** a supported external procedure declaration and omitted optional arguments
- **WHEN** its recovered call executes through the existing resolver
- **THEN** omitted positions remain omitted rather than becoming Undefined values
- **AND** observed object mutations match native execution
- **AND** the native declared maximum argument count is still enforced

#### Scenario: Native module terminator after the last method
- **GIVEN** a supported native image ending with separate method and module terminators
- **WHEN** its methods are reconstructed
- **THEN** the module terminator is not interpreted as part of the last method
- **AND** additional unsupported trailing commands are rejected

#### Scenario: Computed value precedes a mutating call
- **GIVEN** an image computing an arithmetic value before calling a function that modifies a referenced operand
- **WHEN** the image is reconstructed and executed
- **THEN** the computed value is preserved before the call
- **AND** both the returned value and the modified operand match native execution

#### Scenario: Builtin result precedes operand mutation
- **GIVEN** a supported image invoking a date, string, rounding or type builtin before a later call changes its operand
- **WHEN** the image is reconstructed and executed
- **THEN** the builtin result is captured before operand mutation
- **AND** both the assigned and returned results match native execution

#### Scenario: Native uppercase builtin
- **GIVEN** a supported image containing command 61 with zero operand
- **WHEN** its uppercase expression is reconstructed and executed
- **THEN** Cyrillic and Latin results match native execution
- **AND** the result precedes a subsequent reference-mutating call
- **AND** invalid operands and missing arguments are rejected explicitly

#### Scenario: Native right substring and blank-string builtins
- **GIVEN** a supported image containing commands 58 or 65 with zero operands
- **WHEN** the expressions are reconstructed and executed
- **THEN** their results match native right-substring and blank-string evaluation
- **AND** results are preserved before a later call modifies their referenced operand
- **AND** invalid operands or insufficient stack arguments fail explicitly

#### Scenario: Native string conversion builtin
- **GIVEN** a supported image containing command 71 with zero operand
- **WHEN** its conversion is reconstructed and executed
- **THEN** conversion uses the existing BSL string builtin
- **AND** its result is preserved before subsequent operand mutation
- **AND** invalid operands or missing stack arguments are rejected

#### Scenario: Omitted arguments of external functions
- **GIVEN** a supported image calling an external function with omitted argument positions
- **WHEN** the call is reconstructed
- **THEN** omitted positions are preserved in the BSL call
- **AND** argument defaults and runtime validation remain with the ordinary resolver and runtime
- **AND** calls exceeding the native declared argument count are rejected

#### Scenario: Native length and trim builtins
- **GIVEN** a supported image containing commands 53 or 56 with zero operands
- **WHEN** its expressions are reconstructed and executed
- **THEN** string length and trimming use the existing BSL builtins
- **AND** the measured non-BMP string length matches native execution
- **AND** their results precede subsequent operand mutation
- **AND** malformed operands or insufficient arguments are rejected

#### Scenario: Conditional expression followed by statement branches
- **GIVEN** a supported image assigning a conditional expression before an ordinary conditional statement
- **WHEN** both constructs are reconstructed within one method
- **THEN** value branches remain inside their expression
- **AND** subsequent statement branches are recovered independently
- **AND** all measured branch paths and reference changes match native execution

#### Scenario: Counted-loop continuation and break
- **GIVEN** a supported native counted loop containing continuation or break
- **WHEN** its body is reconstructed and executed
- **THEN** continuation reaches counter increment and break reaches loop cleanup
- **AND** accumulated values and reference-counter values match native execution
- **AND** invalid exit targets and unsupported skipped commands are rejected

#### Scenario: Conditional jump taken for a true predicate
- **GIVEN** a measured supported statement branch or conditional expression that jumps on a true predicate
- **WHEN** its complete module is reconstructed and executed
- **THEN** the selected values, reference updates and unselected-branch laziness match native execution
- **AND** invalid alternative or continuation targets reject the complete module

#### Scenario: Supported bodies without source-line instructions
- **GIVEN** a measured image whose supported function or procedure body contains no source-line instructions
- **WHEN** the complete module is reconstructed and executed through the existing BSL pipeline
- **THEN** constant results, statement branches and reference updates match the native receipt
- **AND** invalid function exit operands or exit jump targets still reject the complete module

### Requirement: Explicit rejection of unsupported images
The loader SHALL reject malformed images and unsupported reachable reconstruction forms
with a diagnostic identifying the module and the failing instruction or image
section. It SHALL NOT silently load an empty module, discard unsupported
methods or return fabricated successful results.

#### Scenario: Unsupported instruction
- **GIVEN** an image containing a reachable unsupported instruction
- **WHEN** its module is loaded
- **THEN** loading fails with the module identity and instruction location
- **AND** no partial recovered body is exposed as a complete module

#### Scenario: Next-command jump with expression state
- **GIVEN** a measured supported jump to the immediately following command with a value or reference target on the expression stack
- **WHEN** the complete module is reconstructed and executed
- **THEN** values, conditional paths and reference updates match native execution
- **AND** other jump targets retain their validation and unsupported forms remain diagnostics

#### Scenario: Mutually referencing unreachable regions
- **GIVEN** measured regions skipped from every executable entry but containing jumps into each other
- **WHEN** complete module reachability is checked before reconstruction
- **THEN** those dead references do not become executable entries
- **AND** all native control results are preserved
- **AND** an actual entry or an executable incoming path keeps the destination protected

#### Scenario: Measured constant boolean prefixes
- **GIVEN** supported native constant boolean literals, their negation or equality before a conditional jump
- **WHEN** impossible edges are normalized and guarded regions are reconsidered
- **THEN** all native control results are preserved
- **AND** remaining executable incoming edges and variable predicates still prevent unsafe omission

#### Scenario: Supported method-local label jump
- **GIVEN** a measured forward label jump or structured loop back edge in a supported method with a local label table
- **WHEN** the complete native module is reconstructed
- **THEN** the target resolves through that method's local table and execution matches native results
- **AND** an unused global label table does not change the method's selected target
- **AND** missing referenced labels or invalid executable target boundaries produce an explicit module diagnostic

#### Scenario: Native-proven unreachable instruction region
- **GIVEN** a measured side-effect-free constant predicate or forward jump that makes an instruction region unreachable from every relevant entry
- **WHEN** its supported complete module is reconstructed
- **THEN** omitting that region preserves all native receipt values and reference effects
- **AND** all declarations and executable entry points are retained
- **AND** an incoming control-flow edge or nonconstant predicate prevents unsupported instructions from being silently omitted
- **AND** malformed table records and invalid executable boundaries still reject the module

#### Scenario: Invalid image reference
- **GIVEN** an executable path referencing a nonexistent constant, variable, procedure or branch target
- **WHEN** its module is loaded
- **THEN** loading fails explicitly at the invalid image reference

#### Scenario: Measured explicit boolean conversion
- **GIVEN** a supported native image converting a value to a boolean
- **WHEN** the complete module is reconstructed and executed through open-bsl
- **THEN** measured boolean, numeric and boolean-word inputs produce the native boolean type and value
- **AND** measured unsupported strings, undefined and null raise a catchable conversion error
- **AND** reference assignment preserves the converted boolean rather than the original numeric value
- **AND** invalid operands and missing conversion inputs reject the image explicitly

#### Scenario: Measured asynchronous method declarations
- **GIVEN** supported native private and exported asynchronous procedures and functions
- **WHEN** their complete module is reconstructed and executed through open-bsl
- **THEN** native promise return types, reference updates and visibility are preserved
- **AND** the asynchronous declarations use the existing open-bsl execution model
- **AND** unknown flag combinations and invalid body boundaries remain explicit diagnostics

#### Scenario: Measured boolean conversion at a short-circuit boundary
- **GIVEN** a supported native conjunction or disjunction whose right operand ends with explicit boolean conversion
- **WHEN** its complete module is reconstructed and executed
- **THEN** all native branch results and right-operand reference effects are preserved
- **AND** the unselected right operand is never evaluated
- **AND** malformed conversion operands and invalid join targets reject the image

#### Scenario: Pure constant short-circuit predicate before unreachable code
- **GIVEN** a measured conjunction or disjunction of boolean literals followed by a conditional branch
- **WHEN** complete-module reachability and reconstruction are performed
- **THEN** the four native constant control outcomes are preserved
- **AND** unreachable instructions do not become executable through the unselected logical edge
- **AND** nonconstant values, conflicting incoming facts, executable entries and executable external edges remain protected


#### Scenario: Measured collection loop without a line marker and with a back-edge trampoline
- **GIVEN** a measured collection loop without a source-line marker before its iterator check and with a single extra jump on its back edge
- **WHEN** its image is reconstructed and executed through the facade
- **THEN** mixed, empty, skipped and uninterrupted collections match the native return and parameter receipts
- **AND** continuation reaches the same iterator check
- **AND** malformed iterator checks, back-edge targets and cleanup boundaries are rejected


#### Scenario: Measured collection-loop terminal jump to the preceding back edge
- **GIVEN** a measured collection loop whose terminal jump targets its preceding validated back edge
- **WHEN** its image is reconstructed and executed
- **THEN** collection traversal and reference effects match all native control receipts
- **AND** continuation reaches the validated iterator check
- **AND** a target to the preceding assignment or another iterator instruction is rejected


#### Scenario: Measured renamed iterator storage
- **GIVEN** a measured collection loop with a renamed private iterator variable
- **WHEN** the image is reconstructed
- **THEN** traversal and parameter effects match the native receipt independently of that private name
- **AND** invalid storage flags, inconsistent iterator slots and body access to hidden iterator storage are rejected


#### Scenario: Measured native number conversion
- **GIVEN** a supported native numeric conversion of numbers, numeric strings or Boolean values
- **WHEN** its recovered body executes through the facade
- **THEN** the numeric type and value match the native receipt
- **AND** unsupported values raise catchable errors
- **AND** assignment through a reference parameter receives the converted value
- **AND** missing operands and nonzero reserved command operands are rejected


#### Scenario: Compact conditional collection-loop break
- **GIVEN** a measured conditional break without a redundant forward join inside a supported collection loop
- **WHEN** its image is reconstructed and executed
- **THEN** the selected branch exits the nearest loop and the other branch continues traversal
- **AND** parameter effects and return values match the native receipt
- **AND** a jump to an unvalidated cleanup target is rejected


#### Scenario: Measured procedure early exit directly to the end command
- **GIVEN** a measured procedure early exit targeting its terminal end command without a required line marker
- **WHEN** the reconstructed procedure executes
- **THEN** reference updates preceding the exit are retained and subsequent statements are skipped
- **AND** jumps into other procedure bodies or a nonterminal instruction are rejected


#### Scenario: Compact conditional procedure early return
- **GIVEN** a measured conditional procedure return whose validated exit jump also closes the selected branch
- **WHEN** the reconstructed body executes
- **THEN** the selected path returns with its prior reference effects and the other path retains its own effects
- **AND** the current procedure exit is still validated
- **AND** unknown commands following the return are not silently discarded


#### Scenario: Measured unreachable branch join after procedure return
- **GIVEN** a measured branch containing a validated procedure return followed by a proven unreachable join jump into the alternative body
- **WHEN** the module is reconstructed and executed
- **THEN** the unselected path remains complete and reference effects match native execution
- **AND** reachable incoming paths and other declared entries prevent removal of executable joins
- **AND** unknown commands in either actual branch remain rejected


#### Scenario: Measured single-jump branch and back-edge trampolines
- **GIVEN** a measured false-condition branch or backward loop jump targeting one unconditional jump in the same method
- **WHEN** that target is normalized before structured recovery
- **THEN** execution and reference effects match the native receipt
- **AND** the trampoline command is preserved for any other incoming paths
- **AND** cross-method targets, self cycles and unmeasured jump chains remain unsupported


#### Scenario: Collection iteration ending in continuation or break
- **GIVEN** a measured collection iteration with a false-condition branch to the validated iterator check and a selected branch leaving the loop
- **WHEN** its image is reconstructed and executed
- **THEN** all parameter effects and return values match the independent native receipt
- **AND** proven skipped commands do not erase the structural loop back edge or the break itself
- **AND** unknown or foreign iterator-check targets are rejected


#### Scenario: Known unreachable conditional prefix after a procedure exit
- **GIVEN** a measured known conditional prefix following a validated unconditional procedure exit and unreachable from all declared entries
- **WHEN** its complete module is reconstructed
- **THEN** the real alternative branch remains complete and reference effects match native results
- **AND** another entry or executable incoming path prevents omission
- **AND** unknown instructions after the return remain explicitly rejected


#### Scenario: Measured procedure with a shared loop body
- **GIVEN** a supported native procedure whose loop body appears between an earlier return path and the loop initialization
- **WHEN** its complete body is reconstructed and executed through the existing facade
- **THEN** early-return and looping reference effects match both native image and independently authored BSL receipts
- **AND** executable shared blocks are retained
- **AND** invalid targets, unknown instructions and unsupported operand-stack transfers produce explicit errors


#### Scenario: Measured empty handler in a shared-block procedure
- **GIVEN** a measured empty try-and-handler region in a supported shared-block procedure
- **WHEN** its body is reconstructed and executed through the facade
- **THEN** reference effects match the native receipt
- **AND** the handler remains an ordinary BSL try construct
- **AND** incoming jumps into the interior or malformed handler boundaries are rejected

#### Scenario: Measured left substring instruction
- **GIVEN** a supported image using the measured left substring instruction
- **WHEN** it is reconstructed through the existing BSL runtime
- **THEN** ASCII, Cyrillic, zero-length and overlong requests match the native receipt
- **AND** a nonzero instruction operand or missing expression operand is rejected

#### Scenario: Measured backward boolean assignment join
- **GIVEN** a measured procedure whose true and false expressions join at an earlier shared assignment
- **WHEN** the image is reconstructed and called through the facade
- **THEN** both reference updates match the native receipt
- **AND** reconstruction uses ordinary BSL assignment and conditional expression
- **AND** malformed targets and unsupported incoming paths are explicitly rejected

#### Scenario: Measured empty handler before function return
- **GIVEN** a supported function with measured sealed empty handlers and a validated terminal return
- **WHEN** its source is reconstructed and executed through the facade
- **THEN** all four substring results match the native receipt
- **AND** empty handlers remain ordinary BSL try constructs
- **AND** malformed return tails and unsupported instructions are rejected

#### Scenario: Measured dynamic private function lookup
- **GIVEN** a supported image whose exported function dynamically evaluates a private function call
- **WHEN** reconstructed source executes through the existing facade
- **THEN** the numeric result and type match the native receipt
- **AND** dynamic lookup uses the existing compiler and runtime scope
- **AND** nonzero instruction operands and missing input expressions are rejected

### Requirement: Representable private async constant function
A supported complete native image with a private parameterless async constant-return function SHALL preserve its asynchronous declaration through the existing BSL compiler and VM.

#### Scenario: Local dynamic private async lookup
- **GIVEN** the independently measured private parameterless async constant-return function and its synchronous exported wrapper
- **WHEN** the wrapper evaluates the private function name locally
- **THEN** the result has the native promise type
- **AND** malformed method boundaries, parameter tables and unsupported async bodies reject the whole image

### Requirement: Measured boolean conversion operand
The image loader SHALL recover the measured boolean conversion instruction
with operand 1393 through the existing BSL conversion function. It SHALL
preserve the complete native control image and reject missing input values.

#### Scenario: Nonzero operand in a complete image
- **GIVEN** the measured two-function image with boolean conversion operands 1393
- **WHEN** the loader recovers and executes both exported functions
- **THEN** their results match the independent native true and false receipt
- **AND** this measured operand does not enable unmeasured operands

### Requirement: Measured value parameter slot zero
The loader SHALL preserve the measured value parameter with flag 13 and
slot 0 when its full default-parameter table is present. It SHALL use the
existing BSL value parameter semantics without enabling unknown flags.

#### Scenario: Required value parameter with slot zero
- **GIVEN** the complete measured single-function image with flag 13, slot 0 and a required parameter entry
- **WHEN** its exported function is called with 42
- **THEN** its result is 42
- **AND** omission of the required argument is rejected

### Requirement: Measured shared constant-return tail
The loader SHALL preserve the measured ordinary function whose initial
jump reaches the constant-return body of a private parameterless async
function. It SHALL retain both declarations and their distinct result kinds
through the existing compiler and VM. Unsupported overlapping forms SHALL
remain explicit refusals.

#### Scenario: Ordinary and async entries share a constant tail
- **GIVEN** the complete measured overlapping-tail image and independent native receipt
- **WHEN** both exported wrappers execute through open-bsl
- **THEN** the ordinary call returns 42 and the dynamic private async call returns a promise
- **AND** the private declaration and its body are retained
- **AND** malformed tails and executable entries in the skipped prefix are not discarded

### Requirement: Measured integer-part instruction
The loader SHALL recover the measured zero-operand integer-part instruction
through the existing BSL integer-part function. It SHALL preserve native
results, conversion failures, reference assignment and evaluation order.

#### Scenario: Integer part values and reference assignment
- **GIVEN** the complete measured integer-part image and its native receipt
- **WHEN** positive and negative fractions, a large number, booleans, strings, Undefined and Null are supplied
- **THEN** values, numeric result types and caught failures match the native receipt
- **AND** the reference procedure updates its argument to the measured integer part

#### Scenario: Result is fixed before a later reference mutation
- **GIVEN** the complete measured integer-part snapshot image
- **WHEN** a later local call changes the original argument
- **THEN** comparison uses the previously computed integer part
- **AND** missing input and unmeasured operands remain refused

### Requirement: Measured initializer and module termination
The loader SHALL recover the measured initializer ending with separate
zero-operand initializer and module termination instructions. It SHALL
preserve initialization before method calls and independent object storage.

#### Scenario: Two terminal instructions after initialization
- **GIVEN** the complete measured initializer image with two final termination instructions
- **WHEN** three object instances are created and their module variables are updated
- **THEN** all seven observations match the independent native receipt
- **AND** both termination instructions are validated
- **AND** corrupt initializer instructions still reject the whole image

### Requirement: Measured structured function exception returns
The decompiler SHALL recover the independent compiler-produced try-return control through the existing BSL compiler and VM, preserving reference effects and rejecting malformed handler boundaries and cleanup tails.

#### Scenario: Body and handler returns
- **WHEN** the protected try-return control calls the first function on normal and exceptional paths
- **THEN** its results and reference effects match native `11|Т` and `22|ТИ`

#### Scenario: Continuation after the handler
- **WHEN** the protected control calls the second function on normal and exceptional paths
- **THEN** its results and reference effects match native `33|ТХ` and `33|ТИХ`

### Requirement: Measured static private procedure flag
The decompiler SHALL recover the measured one- and three-reference-parameter static calls to private procedures with flag 2, preserving private visibility and parameter effects. Unmeasured dynamic invocation and unsupported overlapping bodies SHALL remain explicit errors.

#### Scenario: One reference parameter
- **WHEN** the private-procedure-flag2 control executes through recovered BSL
- **THEN** both native lines match: result and argument are 12

#### Scenario: Three reference parameters
- **WHEN** the private-procedure-three control executes through recovered BSL
- **THEN** all four native lines match: result and first argument are 12 while the second and third remain 6 and 7

#### Scenario: Eval rejects a private procedure used as a function
- **WHEN** the private-procedure-eval-outcome control evaluates a call to the private flag-2 procedure as an expression
- **THEN** the error is catchable and the argument remains 5, matching both native outcome lines

### Requirement: Measured native Execute command
The decompiler SHALL recover the independently compiled native Execute command through the existing BSL dynamic compiler and VM. Unsupported operands, missing input and residual expression state SHALL be rejected.

#### Scenario: Dynamic private procedure updates a reference
- **WHEN** the native-image-execute-compiled control executes its string fragment calling a private procedure
- **THEN** its result and reference argument both match the native value 12

#### Scenario: Execute calls a private flag-2 procedure
- **WHEN** the supplemental private-procedure-execute image changes only the private declaration flag from 0 to 2 in the compiled Execute control
- **THEN** recovered BSL matches both native lines: result and reference argument are 12

### Requirement: Measured shared private procedure tail
The decompiler SHALL recover the measured function jump across a private flag-2 entry sharing a line marker and End. Both declared methods SHALL remain available with their original visibility. Unmeasured overlaps SHALL remain explicit errors.

#### Scenario: Both shared-tail entries execute
- **WHEN** shared-tail-both executes the function and then the private procedure through Execute
- **THEN** all four native result and argument lines equal 5

### Requirement: Measured shared reference body
The decompiler SHALL preserve both declarations and reference effects of the measured function and private flag-2 procedure sharing an increment body. The procedure SHALL remain a procedure; recovery SHALL NOT expose its native return value as a function result. Unsupported frame differences and overlap shapes SHALL fail explicitly.

#### Scenario: Shared increment through both entries
- **WHEN** shared-reference-body runs its public function and invokes the private entry through Execute
- **THEN** all four native result and reference-argument lines equal 12

### Requirement: Measured private reference parameter slot
The decompiler SHALL accept external slot zero for the measured reference parameter of a private flag-2 procedure, preserving its reference effects. Unknown parameter flags and unmeasured external slots SHALL remain explicit errors.

#### Scenario: Reference slot zero
- **WHEN** private-variable-slot-only changes only the private reference parameter external slot from minus one to zero
- **THEN** recovered BSL matches native result and argument values 12
