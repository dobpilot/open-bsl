# Design

Before converting BSL parameter values, enumerate parameter tokens using the existing open-sdbl tokenizer. Match supplied names case-insensitively with folded_eq, stripping only the token ampersand. Strings and comments do not name parameters. Convert only referenced values and leave the original QueryRequest untouched. Missing referenced parameters remain the SDK diagnostic; the SDK's own unused-parameter contract remains strict. Do not rewrite BSL or SQL and do not introduce another lexer.

Native evidence: own 1C 8.3.27.2342 control unused-query-parameters.oracle confirms unused UUID, unused value table and parameters retained after changing query text are accepted; a missing used parameter is refused. The own native control changes only a form module and does not insert catalog rows or open original databases.
