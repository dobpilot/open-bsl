## ADDED Requirements

### Requirement: HTML document constructors
The component SHALL resolve `ДокументHTML` / `HTMLDocument` through the
ordinary constructor registry with zero arguments or one string title.
The result SHALL have HTML document identity. Supplying a title SHALL
initialize the document title rather than silently discard it.

#### Scenario: Construct the original helper document
- **WHEN** original BSL constructs `Новый ДокументHTML`
- **THEN** construction succeeds with an HTML document value
- **AND** its root is `html` with no children and its body is undefined

#### Scenario: Construct with a title
- **WHEN** BSL supplies a string title
- **THEN** the document retains that title in its initialized structure
- **AND** the initialized structure contains `head/title` before the body

#### Scenario: Empty title
- **WHEN** the supplied title is an empty string
- **THEN** the root has no children, as with the zero-argument constructor

#### Scenario: Initial document children and parent links
- **WHEN** BSL inspects a newly constructed HTML document
- **THEN** the document has two children in order: a DOM document type
  definition named `html`, then the HTML root element named `html`
- **AND** both children have the document as their parent
- **AND** the document's name is `#document` and its parent is undefined

### Requirement: HTML body and text construction
The HTML document SHALL create an element and text node and permit
assignment of its body followed by appending the text as a body child.
Node identity and document ownership SHALL remain consistent through
these operations. Ordinary XML documents SHALL retain their prior
property and method behavior.

#### Scenario: Build the native helper body
- **WHEN** the host script creates `body`, assigns `Тело`, creates text
  `TimelistRecognizeResult` and appends it to that body
- **THEN** the document body contains exactly that text node as one child

#### Scenario: Replace the body
- **WHEN** the document receives a second own body element
- **THEN** that element becomes the body and preserves its identity
- **AND** the old body's parent becomes undefined

### Requirement: String-backed HTML DOM output
The component SHALL expose `ЗаписьHTML` / `HTMLWriter`, permit string
output initialization and closing, and accept it as a target of
`ЗаписьDOM.Записать`. HTML formatting SHALL reproduce the native
control while ordinary XML serialization remains unchanged.

#### Scenario: Serialize the original helper document
- **WHEN** the document body contains text `TimelistRecognizeResult`
  and is written through the DOM writer into a string-backed HTML writer
- **THEN** closing returns `<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.0 Transitional//EN"><html><body>TimelistRecognizeResult</body></html>`
- **AND** the result contains exactly 111 characters with no newline

#### Scenario: Write title, text and empty elements
- **WHEN** a document contains title `Заголовок <&>` and body text `<&>"`
  followed by empty `br` and `div` elements
- **THEN** title and text escape `<`, `&` and `>`
- **AND** the text quote remains literal
- **AND** the empty elements are written as `<br>` and `<div></div>`

#### Scenario: Keep existing XML output
- **WHEN** an existing XML DOM is written through `ЗаписьXML`
- **THEN** its declaration, escaping, indentation and empty-element
  behavior remain unchanged

#### Scenario: Close an unopened or already closed HTML writer
- **WHEN** BSL calls `Закрыть()` before initializing a string target
  or calls it again after closing the target
- **THEN** the result is undefined without an exception

#### Scenario: Close an initialized empty target
- **WHEN** BSL initializes a string target and closes it without writing
- **THEN** the result is an empty string

#### Scenario: Reset an open string target
- **WHEN** BSL calls `УстановитьСтроку()` again after writing a document
- **THEN** the target is reset and closing returns an empty string

#### Scenario: Reject a second document in one target
- **WHEN** BSL writes a second HTML document without resetting the target
- **THEN** writing fails and the first document's output remains intact

### Requirement: Explicit unsupported HTML operations
An HTML operation whose object model is not implemented SHALL fail
explicitly. HTML parsing SHALL NOT be replaced by a successful XML parse
or a fabricated empty document, and output SHALL NOT silently drop nodes.

#### Scenario: Reach an unsupported HTML operation
- **WHEN** configuration execution calls an unsupported HTML operation
- **THEN** it receives an explicit runtime error without a fabricated result
