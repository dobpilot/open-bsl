# Compact pressured configuration method frames

Original BUH compilation fails in ЗаполнениеБухгалтерскойОтчетностиУпрощенная.ЗаполнитьМПФормаОтчета2012Кв4_БалансПоГрафе2012Кв4. Its243 source locals fit the frame, but a thirteen-node expression needs thirteen more temporary registers. Existing slot allocation compiles the same method using80local slots. The automatic large-source-frame path currently excludes it.

Retry eligible configuration methods with existing slot allocation only after ordinary bytecode generation exhausts temporary registers. Preserve successful ordinary compilation, standalone limits and the existing255-register format.
