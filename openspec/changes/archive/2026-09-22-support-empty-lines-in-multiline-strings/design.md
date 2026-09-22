# Design

The Syntax Assistant 8.3.27.2342 document `platform-syntax-help:platform-guides:def_String` permits whitespace and line breaks between the fragments of a multiline string. Focused measurements on 1C:Enterprise 8.3.27.2342 used one empty physical line before the next `|` fragment. Both an LF source file and a CRLF source file produced the two active fragments separated by exactly one LF; the empty line contributed nothing.

The string scanner will recognize LF and CRLF as one physical line ending. After an active fragment it will skip horizontal whitespace, complete comment lines, and empty physical lines until it reaches the mandatory `|` of the next active fragment. It will append exactly one LF only after finding that fragment. Other text in the gap retains the existing `ExpectedContinuationBar` diagnostic.

After implementation, the authorized `unf` compile gate passed this location and reached the next independent parser barrier: an identifier where the parser requires `(`. The same three previously known unreadable `.7` resources were skipped.
