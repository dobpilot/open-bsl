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

# Файловые выходы invoice_doc_generator (MXL, PDF, XLSX) — туда же: 1С
# стартует со своим текущим каталогом, и относительный «benchmarks/...»
# разрешится мимо дерева проекта.
RELATIVE_INVOICE_OUTPUT = '"benchmarks/invoice_doc.'
SCRATCH_INVOICE_OUTPUT = '"/tmp/onec-bench-scratch/invoice_doc.'

DECL = re.compile(
    r"^(Процедура|Функция)\s+([A-Za-zА-Яа-яЁё_][\w]*)", re.MULTILINE
)
END = re.compile(r"^(КонецПроцедуры|КонецФункции)\s*$", re.MULTILINE)
# Шапка `//@используй(путь как Псевдоним)` (равноправная форма —
# `//@use(path as Alias)`). Путь — относительно каталога самого
# сценария, кавычки вокруг него необязательны.
USE = re.compile(
    r"^//@(?:use|используй)\(\s*['\"]?(.+?)['\"]?\s+(?:as|как)\s+"
    r"([A-Za-zА-Яа-яЁё_][\w]*)\s*\)\s*$",
    re.MULTILINE,
)
# Цель фонового задания нельзя объявить в модуле формы: такой
# сценарий явно исключает себя из однофайловой платформенной сборки.
SKIP = re.compile(r"^// @skip-1c-combined(?::.*)?$", re.MULTILINE)
TOKEN = re.compile(
    r'"(?:""|[^"])*"|//[^\n]*|[A-Za-zА-Яа-яЁё_][\w]*|\s+|.', re.DOTALL
)
IDENTIFIER = re.compile(r"^[A-Za-zА-Яа-яЁё_][\w]*$")


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


def strip_alias(text, alias):
    """Снимает квалификатор: `Псевдоним.Имя(` -> `Имя(`.

    В однофайловой сборке подключённый модуль склеен в тот же модуль
    платформы, и квалифицированного вызова там быть не может. Работа
    идёт по токенам, а не текстовым regex: псевдоним может встретиться
    внутри строки или комментария, и там его трогать нельзя.
    """
    tokens = TOKEN.findall(text)
    result = []
    folded = alias.casefold()
    i = 0
    while i < len(tokens):
        token = tokens[i]
        if IDENTIFIER.match(token) and token.casefold() == folded:
            rest = [
                k
                for k in range(i + 1, min(i + 7, len(tokens)))
                if not tokens[k].isspace()
            ]
            if (
                len(rest) >= 3
                and tokens[rest[0]] == "."
                and IDENTIFIER.match(tokens[rest[1]])
                and tokens[rest[2]] == "("
            ):
                result.append(tokens[rest[1]])
                result.append("(")
                i = rest[2] + 1
                continue
        result.append(token)
        i += 1
    return "".join(result)


def read_scenario(path):
    """Читает сценарий и подключает объявления модуля из шапки-директивы.

    Возвращает пару (текст, псевдоним). Псевдоним — `None`, когда
    директивы нет.
    """
    text = path.read_text(encoding="utf-8-sig")
    match = USE.search(text)
    if not match:
        return text, None
    source = (path.parent / match.group(1)).resolve()
    if not source.is_relative_to(ROOT) or not source.is_file():
        raise ValueError(f"недопустимая директива в {path}: {match.group(1)}")
    shared = source.read_text(encoding="utf-8-sig")
    declarations, _, _ = split_declarations(shared)
    # Сторонний модуль хранит пробелы на пустых строках. В генерируемый
    # файл они не несут смысла и мешают обычной проверке `git diff`.
    declarations = "\n".join(line.rstrip() for line in declarations.splitlines())
    return declarations + "\n" + USE.sub("", text), match.group(2)


def suffix_identifiers(text, names, suffix):
    """Уникализирует идентификаторы, не затрагивая строки и члены объектов."""
    replacements = {name.casefold(): name + suffix for name in names}
    result = []
    previous = None
    for token in TOKEN.findall(text):
        replacement = replacements.get(token.casefold()) if IDENTIFIER.match(token) else None
        if replacement and previous != "." and (
            previous is None or previous.casefold() != "новый"
        ):
            token = replacement
        result.append(token)
        if not token.isspace() and not token.startswith("//"):
            previous = token
    return "".join(result)


def main():
    only = set(sys.argv[2].split(",")) if len(sys.argv) > 2 else None
    out = []
    scenarios = []
    for path in sorted(BENCH.glob("*.bsl")):
        name = path.stem
        if only and name not in only:
            continue
        source_text = path.read_text(encoding="utf-8-sig")
        if SKIP.search(source_text):
            continue
        text, alias = read_scenario(path)
        if alias:
            text = strip_alias(text, alias)
        text = (
            text.replace(RELATIVE_OUTPUT, SCRATCH_OUTPUT)
            .replace(RELATIVE_EDATA_OUTPUT, SCRATCH_EDATA_OUTPUT)
            .replace(RELATIVE_INVOICE_OUTPUT, SCRATCH_INVOICE_OUTPUT)
            .replace(DATA_PREFIX, ABSOLUTE_DATA_PREFIX)
        )
        decls, body, names = split_declarations(text)
        # Уникализируем имена бенчмарка: два сценария объявляют
        # `CalcНаСервере`, и на уровне модуля они бы столкнулись.
        suffix = "_" + re.sub(r"\W", "_", name)
        # В Connector имена функций совпадают с именами типов, методов и
        # строковых ключей (`ХешированиеДанных`, `ВызватьHTTPМетод`,
        # `Таймаут`). Текстовый regex менял бы и их, поэтому здесь нужен
        # хотя бы минимальный лексический проход по BSL.
        decls = suffix_identifiers(decls, names, suffix)
        body = suffix_identifiers(body, names, suffix)
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
