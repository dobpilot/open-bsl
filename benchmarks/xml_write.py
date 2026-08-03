"""Двойник `xml_write.bsl` на потоковом `XMLGenerator`."""

from io import StringIO
from time import perf_counter

from benchmark_data import PASSES, write_xml_document


started = perf_counter()
total_length = 0
for _ in range(PASSES):
    target = StringIO()
    write_xml_document(target)
    text = target.getvalue()
    total_length += len(text)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"длина документа: {total_length // PASSES}, проходов: {PASSES}")
print(f"{elapsed_ms:.3f}")
