# Двойник regexp.bsl: апстримная реализация regex-benchmark на модуле re —
# те же три шаблона и findall по тому же тексту. `\w` у re юникодный, как и
# у нашего движка; края, где словари «словесных» символов расходятся
# (комбинирующие знаки, White_Space вне ASCII), на этом корпусе счёта не
# меняют.

from time import perf_counter
import re

with open("benchmarks/data/input_regexp.txt", encoding="utf-8") as f:
    data = f.read()

started = perf_counter()
emails = re.findall(r"[\w\.+-]+@[\w\.-]+\.[\w\.-]+", data)
uris = re.findall(r"[\w]+://[^/\s?#]+[^\s?#]+(?:\?[^\s#]*)?(?:#[^\s]*)?", data)
ips = re.findall(
    r"(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}"
    r"(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)",
    data,
)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"почта: {len(emails)}")
print(f"URI: {len(uris)}")
print(f"IP: {len(ips)}")
print(f"{elapsed_ms:.3f}")
