# Символьные устройства Linux в объекте `Файл`

Измерение выполнено напрямую на серверной `localhost/test`, Linux,
1С 8.3.27.2342. Unica не использовалась. Сессия
`/tmp/open-bsl-server-measure-1ig_lmln` завершилась с кодом 0, восстановила
CF и БД; контрольная XML-выгрузка совпала с исходной.

- Исходник `file-object-devices.bsl`: SHA-256
  `c6315760051ae02fc461abb3059da21b2215fbbf8b04cc32f5c9dcce1f0e8c35`.
- Oracle `file-object-devices.platform.txt`: SHA-256
  `692b54e8c08f51ef7aaea48a45a986617c37f259a3d7cedcf97dbbef543ec1e9`.

Проверены `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/random` и
`/dev/urandom`. Для каждого серверная 1С вернула `Существует=Истина`,
`ЭтоФайл=Истина`, `ЭтоКаталог=Ложь` и `Размер=0`. До исправления open-bsl
считал эти объекты иным видом metadata: существование было истинным,
`ЭтоФайл` — ложным, а `Размер` завершался ошибкой.

`SystemFileSystem` теперь относит измеренный `FileTypeExt::is_char_device()`
к файлам наряду с ранее измеренными FIFO и Unix-сокетом. Правило не
распространяется на блочные устройства и остальные специальные Unix-типы:
для них oracle отсутствует. Unit-тест проверяет `/dev/null`, а сквозной тест
сверяет все 22 строки в source/bytecode × plain/optimize.
