"""Проверяет извлечение защищённых образов без изменения исходных артефактов."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
import zlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("vendor", type=Path)
parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
args = parser.parse_args()
sys.path.insert(0, str(args.vendor / "src"))
from v8unpack.container import Container
from v8unpack.MetaObject import MetaObject

root = args.root
results = []
controls = root / "tests/conformance/measure/1c"
inputs = [(name, controls / f"native-image-{name}.Module.bin", False,
           controls / f"native-image-{name}.image") for name in
          ("scoped-foreach-frame", "zero-label-private-path", "nonzero-label-private-path")]
originals = root / "target/diagnostics/readonly-startup"
inputs += [(f"original-{name}", originals / f"original-{name}.module.deflate", True,
            originals / "original-unf-container/image") for name in ("buh", "unf")]
for name, source, compressed, expected in inputs:
    if not source.exists():
        results.append({"fixture": name, "status": "missing"})
        continue
    raw = source.read_bytes()
    data = zlib.decompress(raw, -15) if compressed else raw
    with tempfile.TemporaryDirectory(prefix="open-bsl-v8unpack-check-") as temporary:
        base = Path(temporary)
        folder = base / "module.0"
        container = Container()
        stream = io.BytesIO(data)
        container.read(stream)
        container.extract(str(folder))
        image = (folder / "image").read_bytes()
        assert image == expected.read_bytes(), name
        module = MetaObject()
        module.header["uuid"] = "module"
        module.decode_code(str(base))
        output = base / "output"
        output.mkdir()
        module.write_decode_code(str(output), "module")
        # Проверяем отдельно бинарный путь записи, недостижимый при отсутствии text.
        binary = MetaObject()
        binary.code["obj"] = image
        binary.header["code_encoding_obj"] = "image"
        binary.write_decode_code(str(output), "preserved")
        assert (output / "preserved.obj.image").read_bytes() == image
        result = {
            "fixture": name, "status": "checked", "image_bytes": len(image),
            "image_sha256": hashlib.sha256(image).hexdigest(),
            "container_image_matches": True,
            "decode_code_is_none": module.code.get("obj") is None,
            "decode_code_encoding": module.header.get("code_encoding_obj"),
            "decode_code_emitted_bsl": (output / "module.obj.bsl").exists(),
            "binary_write_preserves_image": True,
        }
        assert result["decode_code_is_none"] and not result["decode_code_emitted_bsl"], name
        results.append(result)
print(json.dumps(results, ensure_ascii=False, indent=2))
