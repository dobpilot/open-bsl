# HTML document output design

## Evidence
Read-only source acquisition locates the helpers at lines 1388 and 1408
of demo's `Таймлист`: `Новый ДокументHTML`, `СоздатьЭлемент("body")`,
assignment of `Тело`, `СоздатьТекстовыйУзел`, `ДобавитьДочерний`, then
`ЗаписьDOM.Записать` to string-backed `ЗаписьHTML`.
Native 8.3.27.2342 controls are saved in
`docs/research/fixtures/html-document/`. Both zero-argument and title
constructors succeed. The original body/text construction produces
exactly 111 characters with an HTML 4.0 Transitional doctype and no
newline. The syntax-helper title-constructor page does not justify
rejecting the measured zero-argument form.

## Placement and reuse
Keep HTML construction beside the existing DOM in `bsl-xml`.
Reuse node identity, ownership and traversal rather than introduce a
second tree. Add HTML document/writer identity and the reached body
surface without exposing those properties on ordinary XML documents.
Reuse the existing text escaping helper and DOM nodes. HTML formatting
uses a separate mode in the DOM serialization module: the XML writer's
declaration and empty-element handling cannot produce the measured HTML
output. This is not a second XML serializer or a replacement of XML output.
The base runtime currently registers an HTMLWriter constructor which
always reports unsupported operation. The implemented constructor must
replace that registration, not compete with it: retain one constructor
name in the component registry, implemented in bsl-xml. The legacy test
which requires construction to fail must be updated only after explicit
approval under the repository's existing-test rule. Constructor codes in
bsl-rt are not renumbered; the removed stub's code stays unused.
Native controls, not assumed XML equivalence, determine HTML escaping,
empty-element output and document initialization. Any unmeasured platform
decision must follow the existing three-part open-question discipline.

## Limits
The first measured helper is in-memory construction and string output.
Do not add browser rendering, network access or file IO. Reading arbitrary
HTML is not implemented by passing it to the XML parser. Additional
reached operations require native controls and explicit implementation;
the full configured-session startup objective remains unfinished until
its required operations work.


## Locale calibration

The original English receipts are retained. The identical structure driver
under /Lru emits Russian booleans and a Russian body type name; static DOM type
localization remains a separate known limitation. Additional drivers make only
Boolean formatting explicitly en. Their /Len native receipts equal the original
English receipts byte for byte, and the CLI replays them without output
normalization. This calibrates host formatting for the HTML-output regression;
it does not claim full Russian DOM type-presentation compatibility.
