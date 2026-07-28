#!/usr/bin/env python3
"""Собирает все сценарии из benchmarks/*.bsl в ОДИН скрипт для платформы.

Зачем один: подъём 1С стоит десятки секунд (создание информационной базы,
загрузка внешней обработки), и платить это за каждый сценарий незачем —
время меряет сам сценарий, а не обёртка.

Как: тело каждого бенчмарка заворачивается в свою процедуру, поэтому его
переменные не сталкиваются с чужими; объявленные бенчмарком процедуры
переезжают на уровень модуля с суффиксом (`CalcНаСервере` есть сразу в
двух). Перед каждым сценарием печатается строка `#имя`, последняя строка
до следующего маркера — миллисекунды: тот же контракт, что у run.sh.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BENCH = ROOT / "benchmarks"

# csv_write* меряют файловый ввод-вывод, а не интерпретатор — их и run.sh
# пропускает.
SKIP = ("csv_write",)

DECL = re.compile(
    r"^(Процедура|Функция)\s+([A-Za-zА-Яа-яЁё_][\w]*)", re.MULTILINE
)
END = re.compile(r"^(КонецПроцедуры|КонецФункции)\s*$", re.MULTILINE)


def split_declarations(text):
    """Делит текст на (объявления, тело) и возвращает имена объявленных."""
    lines = text.splitlines()
    decls, body, names = [], [], []
    i = 0
    while i < len(lines):
        m = DECL.match(lines[i])
        if not m:
            body.append(lines[i])
            i += 1
            continue
        names.append(m.group(2))
        start = i
        while i < len(lines) and not END.match(lines[i]):
            i += 1
        decls.extend(lines[start : i + 1])
        i += 1
    return "\n".join(decls), "\n".join(body), names


def main():
    out = []
    scenarios = []
    for path in sorted(BENCH.glob("*.bsl")):
        name = path.stem
        if name.startswith(SKIP):
            continue
        decls, body, names = split_declarations(
            path.read_text(encoding="utf-8")
        )
        # Уникализируем имена бенчмарка: два сценария объявляют
        # `CalcНаСервере`, и на уровне модуля они бы столкнулись.
        suffix = "_" + re.sub(r"\W", "_", name)
        for n in names:
            pattern = re.compile(rf"\b{re.escape(n)}\b")
            decls = pattern.sub(n + suffix, decls)
            body = pattern.sub(n + suffix, body)
        if decls.strip():
            out.append(decls)
        proc = "Сценарий" + suffix
        scenarios.append((name, proc))
        out.append(f"Процедура {proc}()\n{body}\nКонецПроцедуры")

    calls = "\n".join(
        f'\tСообщить("#{name}");\n\t{proc}();' for name, proc in scenarios
    )
    repeats = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    out.append(f"Для Повтор = 1 По {repeats} Цикл\n{calls}\nКонецЦикла;")

    dest = BENCH / "1c" / "combined.bsl"
    dest.write_text(
        "// СГЕНЕРИРОВАН build-combined.py — править нужно исходные\n"
        "// benchmarks/*.bsl, а не этот файл.\n\n" + "\n\n".join(out) + "\n",
        encoding="utf-8",
    )
    print(f"{dest}: сценариев {len(scenarios)}, повторов {repeats}")


if __name__ == "__main__":
    main()
