#!/usr/bin/env python3
"""Распаковать контент-потоки PDF и напечатать их вместе с /MediaBox.

Разбирать PDF по-настоящему тут незачем: снятые платформой файлы устроены
единообразно, и всё, что нужно для съёмки раскладки, — это операторы `re`,
`Td` и `Tf` из потоков страниц плюс размер страницы. Поэтому скрипт ищет
`stream ... endstream` подряд по файлу и распаковывает те куски, которые
распаковываются как zlib; остальные (шрифт, `/ToUnicode`) пропускает.

    python3 tests/conformance/pdf/dump-pdf-streams.py probe-grid.pdf

Это ИНСТРУМЕНТ СЪЁМКИ, а не часть сборки: рабочее пространство остаётся
без внешних зависимостей, python3 нужен только тому, кто пересматривает
снятые файлы.
"""

import re
import sys
import zlib

for path in sys.argv[1:]:
    data = open(path, "rb").read()
    print(f"=== {path} ({len(data)} байт)")
    for m in re.finditer(rb"/MediaBox\s*\[([^\]]*)\]", data):
        print("MediaBox:", m.group(1).decode("latin-1").strip())
    for m in re.finditer(rb"stream\r?\n", data):
        start = m.end()
        end = data.find(b"endstream", start)
        if end < 0:
            continue
        try:
            body = zlib.decompress(data[start:end])
        except zlib.error:
            continue
        try:
            text = body.decode("latin-1")
        except UnicodeDecodeError:
            continue
        # Поток страницы отличается от прочих тем, что состоит из операторов.
        if " re" not in text and " Td" not in text:
            continue
        print(f"--- поток на смещении {start}, {len(body)} байт после распаковки")
        print(text)
