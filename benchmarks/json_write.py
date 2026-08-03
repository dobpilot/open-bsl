"""Двойник `json_write.bsl` на стандартном модуле `json`."""

import json
from time import perf_counter

from benchmark_data import PASSES, make_records


started = perf_counter()
text = ""
total_length = 0
for _ in range(PASSES):
    text = json.dumps(
        make_records(),
        ensure_ascii=False,
        separators=(",", ":"),
    )
    total_length += len(text)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"длина документа: {total_length // PASSES}, проходов: {PASSES}")
print(f"{elapsed_ms:.3f}")
