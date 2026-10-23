# Fit configuration method name indices

Original BUH compilation fails with TooManyNames in Report.СведенияОбОтклоненияхИСМП.ObjectModule.ПриСозданииНаСервере. A retained public-API reproducer shows that 65536 property names can compile alone, but a method introduced by another module cannot: property names use u32 NameId while open method operands use u16.

Allow configuration compilation to place the finite set of statically required method/type names within the existing operand range while preserving a single catalog-wide name namespace. Keep successful ordinary compilation, shared immutable storage, the bytecode format and crate dependencies unchanged. This change does not promise arbitrary new dynamic method names after the catalog has been frozen.
