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

# Сценарии с файловым выводом открывают "test.csv" ОТНОСИТЕЛЬНО текущего
# каталога, а у платформы он свой и не наш. Путь поэтому переписывается на
# тот же каталог, куда пишет run.sh: работа от этого не меняется, а
# получившийся файл можно сличить с нашим — 1С и мы обязаны положить на
# диск одни и те же байты.
RELATIVE_OUTPUT = '"test.csv"'
SCRATCH_OUTPUT = '"/tmp/onec-bench-scratch/csv_write.1c.out"'

# По той же причине переписывается каталог входных данных: сценарии
# (`simple_parquet_reader`, `edata_writer`) берут файлы от корня дерева
# (`benchmarks/data/...`), а платформа стартует со своим текущим каталогом.
# Префикс превращается в абсолютный — те же файлы, та же работа.
DATA_PREFIX = '"benchmarks/data/'
ABSOLUTE_DATA_PREFIX = f'"{ROOT / "benchmarks" / "data"}/'

# Файловый выход edata_writer — в тот же scratch-каталог, что и у
# csv_write: получившийся XML можно сличить с нашим побайтно.
RELATIVE_EDATA_OUTPUT = '"benchmarks/edata_writer.xml"'
SCRATCH_EDATA_OUTPUT = '"/tmp/onec-bench-scratch/edata_writer.1c.xml"'

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
    only = set(sys.argv[2].split(",")) if len(sys.argv) > 2 else None
    out = []
    scenarios = []
    for path in sorted(BENCH.glob("*.bsl")):
        name = path.stem
        if only and name not in only:
            continue
        text = (
            path.read_text(encoding="utf-8")
            .replace(RELATIVE_OUTPUT, SCRATCH_OUTPUT)
            .replace(RELATIVE_EDATA_OUTPUT, SCRATCH_EDATA_OUTPUT)
            .replace(DATA_PREFIX, ABSOLUTE_DATA_PREFIX)
        )
        decls, body, names = split_declarations(text)
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
