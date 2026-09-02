#!/usr/bin/env python3
"""Сборка `.vsix` из `package.json` рядом с этим скриптом.

Просто положить папку в каталог расширений недостаточно: начиная с
некоторой версии VS Code берёт список пользовательских расширений из
реестра `extensions.json`, а не из содержимого каталога, и неучтённую
папку молча пропускает. Проверено на Code OSS 1.131: каталог с одной
папкой её показывает, тот же каталог с реестром, где её нет, — уже нет.

`.vsix` — это zip с манифестом, поэтому здесь не нужно ни `vsce`, ни
Node.js, ни единой внешней зависимости.

    python3 docs/reference/editors/vscode/pack.py [выходной-файл]
    code-oss --install-extension open-bsl-debug.vsix
"""

import json
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

HERE = Path(__file__).parent
MANIFEST = json.loads((HERE / "package.json").read_text(encoding="utf-8"))

CONTENT_TYPES = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json"/>
  <Default Extension="vsixmanifest" ContentType="text/xml"/>
</Types>
"""

VSIX_MANIFEST = """<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="{name}" Version="{version}" Publisher="{publisher}"/>
    <DisplayName>{display}</DisplayName>
    <Description xml:space="preserve">{description}</Description>
    <Categories>Debuggers</Categories>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
  </Assets>
</PackageManifest>
"""


def main() -> None:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
        f"{MANIFEST['name']}-{MANIFEST['version']}.vsix"
    )
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as vsix:
        vsix.writestr("[Content_Types].xml", CONTENT_TYPES)
        vsix.writestr(
            "extension.vsixmanifest",
            VSIX_MANIFEST.format(
                name=MANIFEST["name"],
                version=MANIFEST["version"],
                publisher=MANIFEST["publisher"],
                display=escape(MANIFEST["displayName"]),
                description=escape(MANIFEST["description"]),
            ),
        )
        vsix.write(HERE / "package.json", "extension/package.json")
    print(out)


if __name__ == "__main__":
    main()
