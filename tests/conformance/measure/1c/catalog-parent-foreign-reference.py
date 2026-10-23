"""Вторая ссылка справочника для проб родителя.

Копирует уже собранный `ЗамерВременныхТаблиц` под именем `ЗамерЧужойСсылки`
с детерминированно пересчитанными UUID и регистрирует копию в составе
конфигурации: проба проверяет присваивание ссылки чужого типа справочника.
Аргумент — каталог собираемой выгрузки.
"""
import pathlib
import re
import sys
import uuid
cfg = pathlib.Path(sys.argv[1])
source = cfg / "Catalogs/ЗамерВременныхТаблиц.xml"
text = source.read_text().replace("ЗамерВременныхТаблиц", "ЗамерЧужойСсылки")
text = re.sub(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}",
              lambda match: str(uuid.uuid5(uuid.NAMESPACE_URL, "open-bsl-parent/" + match.group())), text)
(cfg / "Catalogs/ЗамерЧужойСсылки.xml").write_text(text)
root = cfg / "Configuration.xml"
root.write_text(root.read_text().replace("<ChildObjects>", "<ChildObjects>\n\t\t\t<Catalog>ЗамерЧужойСсылки</Catalog>", 1))
