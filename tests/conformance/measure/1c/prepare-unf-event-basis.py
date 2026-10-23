"""Подготовка изолированной пробы исходного определяемого типа УНФ.

Декларация типа сохранена из выгрузки платформы без изменения состава.
Ссылочные объекты заменены минимальными объектами с теми же именами:
проба проверяет типы и UUID, не прикладные модули или данные УНФ.
"""

from pathlib import Path
import re
import shutil
import sys
import uuid
import xml.etree.ElementTree as ET

source = Path(__file__).parent
destination = Path(sys.argv[1])
declaration = source / "unf-event-basis-defined-type.xml"
namespaces = {
    "m": "http://v8.1c.ru/8.3/MDClasses",
    "v": "http://v8.1c.ru/8.1/data/core",
}
types = [
    node.text
    for node in ET.parse(declaration).findall(".//m:Properties/m:Type/v:Type", namespaces)
]
assert len(types) == 56 and len(set(types)) == 56
assert sum(value.startswith("cfg:DocumentRef.") for value in types) == 54
assert sum(value.startswith("cfg:CatalogRef.") for value in types) == 2

configuration = destination / "Configuration.xml"
entries = ["<DefinedType>ОснованиеСобытия</DefinedType>"]
for value in types:
    kind, name = value.removeprefix("cfg:").split(".", 1)
    if kind == "DocumentRef":
        directory, tag, original = "Documents", "Document", "ЗамерДокумента"
        template = source / "cfg-src-document-reference" / directory / f"{original}.xml"
    else:
        directory, tag, original = "Catalogs", "Catalog", "ЗамерВременныхТаблиц"
        template = source / "cfg-src-temp-tables" / directory / f"{original}.xml"
    text = template.read_text().replace(original, name)
    text = re.sub(
        r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}",
        lambda match: str(uuid.uuid5(uuid.NAMESPACE_URL, f"unf-basis/{value}/{match[0]}"))
        if uuid.UUID(match[0]).int else match[0],
        text,
    )
    (destination / directory).mkdir(exist_ok=True)
    (destination / directory / f"{name}.xml").write_text(text)
    entries.append(f"<{tag}>{name}</{tag}>")

(destination / "DefinedTypes").mkdir(exist_ok=True)
shutil.copyfile(declaration, destination / "DefinedTypes/ОснованиеСобытия.xml")
text = configuration.read_text()
assert text.count("<ChildObjects>") == 1
configuration.write_text(text.replace("<ChildObjects>", "<ChildObjects>\n" + "\n".join(entries), 1))

probe = (source / "document-fill-mixed-references.xml").read_text()
probe = probe.replace(
    "<v8:Type>cfg:DocumentRef.ЗамерДокумента</v8:Type><v8:Type>cfg:CatalogRef.ЗамерВременныхТаблиц</v8:Type>",
    "<v8:TypeSet>cfg:DefinedType.ОснованиеСобытия</v8:TypeSet>",
)
assert probe.count("<v8:TypeSet>") == 2
(destination / "Documents/ЗамерДокумента.xml").write_text(probe)
