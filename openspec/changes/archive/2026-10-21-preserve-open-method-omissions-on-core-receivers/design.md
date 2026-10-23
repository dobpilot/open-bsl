# Design
The compiler already selects CallObjectMethod/CallObjectProcedure for names absent from BuiltinMethod. The resolver must use the existing ResolvedArg::Default representation for that same case, independently of core-receiver inference. Only a known closed builtin can use resolve_required_args. Existing measured Copy/Insert exceptions and background Execute stay unchanged. TextWriter.Open execution remains unsupported until separately implemented; a dormant original declaration can compile without simulating a writer.

Native platform 8.3.27.2342 accepts both writer aliases with omissions and reports an unknown structure method only at execution. No new platform assumptions are introduced. No VM or hot-loop changes are needed.
