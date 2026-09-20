"""Проверки контекста объявлений в оболочке платформенного измерителя."""

import pathlib
import subprocess
import sys
import tempfile
import unittest


class FormModuleTests(unittest.TestCase):
    def generate(self, source):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            script = root / "probe.bsl"
            module = root / "Module.bsl"
            script.write_text(source, encoding="utf-8")
            subprocess.run(
                [sys.executable, str(pathlib.Path(__file__).with_name("gen-form-module.py")),
                 str(script), str(module), str(root / "result.txt")],
                check=True, capture_output=True, text=True,
            )
            return module.read_text(encoding="utf-8")

    def test_client_module_variables_have_client_context(self):
        module = self.generate(
            "Перем Счетчик;\nПерем Завершено;\n"
            "Асинх Процедура Проба()\nПерем Локальная;\nКонецПроцедуры\n"
            "Счетчик = 0;\n"
        )
        self.assertTrue(module.startswith(
            "&НаКлиенте\nПерем Счетчик;\n&НаКлиенте\nПерем Завершено;\n"
        ))
        self.assertIn("Асинх Процедура Проба()\nПерем Локальная;", module)
        self.assertIn("\tСчетчик = 0;", module)

    def test_server_module_variables_keep_existing_context(self):
        module = self.generate("Перем Счетчик;\nСчетчик = 0;\n")
        self.assertTrue(module.startswith("Перем Счетчик;\n"))
        self.assertIn("&НаСервере\nПроцедура ВыполнитьТестНаСервере()", module)

    def test_explicit_server_suffix_applies_inside_a_client_probe(self):
        module = self.generate(
            "// @onec-server-without-context НаСервере\n"
            "Функция СоздатьНаСервере()\nВозврат 7;\nКонецФункции\n"
            "Асинх Процедура Клиентская()\nКонецПроцедуры\n"
            "Сообщить(СоздатьНаСервере());\n"
        )
        self.assertIn("&НаСервереБезКонтекста\nФункция СоздатьНаСервере()", module)
        self.assertIn("&НаКлиенте\nАсинх Процедура Клиентская()", module)
        self.assertNotIn("&НаКлиенте\nФункция СоздатьНаСервере()", module)


if __name__ == "__main__":
    unittest.main()
