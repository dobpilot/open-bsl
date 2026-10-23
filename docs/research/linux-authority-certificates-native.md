# Сертификаты удостоверяющих центров Linux

Замер выполнен 2026-10-02 на платформе 8.3.27.2342, локаль `ru`, в отдельной файловой базе `/tmp/open-bsl-linux-ca-096-ib`. Конфигурация и данные исходных demo/buh/unf не изменялись.

Скрипт и эталон находятся в `tests/conformance/measure/1c/measure-linux-authority-certificates.bsl` и соседнем `.platform.txt`. Создание возвращает тип с представлением `LinuxCertificationAuthorityCertificates`; сравнение с `Тип("LinuxCertificationAuthorityCertificates")` истинно. Передача этого объекта вторым аргументом `ЗащищенноеСоединениеOpenSSL` успешна. Запрос HTTPS не выполнялся: доверие сертификату сервера, проверка имени и момент чтения системного хранилища этой пробой не измерены.

Синтакс-помощник установленной 8.3.27.2342, русская локаль:

- `platform-syntax-help:syntax-context:objects/catalog63/catalog578/catalog2014/LinuxCertificationAuthorityCertificates/ctors/ctor_Auto.html` — конструктор без аргументов.
- `platform-syntax-help:syntax-context:objects/catalog63/catalog578/catalog2014/LinuxCertificationAuthorityCertificates.html` — PEM-сертификаты из `/etc/ssl/certs`.
- `platform-syntax-help:syntax-context:objects/catalog63/catalog578/catalog2014/OSCertificationAuthorityCertificates.html` — эквивалентность источника ОС источнику Linux на Linux; это не тождество типов.

Первая попытка замера на отсутствующем дисплее `:1` не запустила конфигуратор и не является свидетельством о семантике. Повтор на фактическом `:0` завершился с кодом 0 и тремя строками результата.
