"""Двойник `str_concat.bsl` с последовательной конкатенацией строк."""

from time import perf_counter


PIECE = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789"


started = perf_counter()
text = ""
for _ in range(10):
    text = ""
    for _ in range(3_000):
        text = text + PIECE
elapsed_ms = (perf_counter() - started) * 1_000

print(f"длина: {len(text)}")
print(f"{elapsed_ms:.3f}")
