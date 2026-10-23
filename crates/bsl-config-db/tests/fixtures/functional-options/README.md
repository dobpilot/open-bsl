# Приобретённые источники функциональных опций

Ресурсы Config из нативных контрольных CF платформы 8.3.27.2342.
Экстракция SDK 0.9.12 скопирована из его runtime-options-0912 фикстур.
Независимые исходные CF/XML находятся в open-bsl tests/conformance/measure/1c:
functional-option-native, functional-option-catalog-native,
functional-option-register-native. SHA256SUMS фиксирует сжатые ресурсы
и полные списки, без пересериализации Config. Тесты используют существующий
SDK parser/resolver и проверяют только проекцию приобретённых данных.
