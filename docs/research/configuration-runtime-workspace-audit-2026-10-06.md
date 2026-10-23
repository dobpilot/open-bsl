# Проверка workspace после добавления бизнес-процессов

Проверяется весь текущий рабочий снимок, не только новые тесты бизнес-процесса. Проверка не подтверждает выполнение оригинальных startup demo/buh/unf.

- `cargo clippy --workspace --all-targets --offline -- -D warnings`: успешно, 37.61 с; лог `/tmp/open-bsl-workspace-clippy-audit-1006.log`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --offline`: успешно, 32.10 с; лог `/tmp/open-bsl-workspace-doc-audit-1006.log`.
- Обычный `cargo test --workspace --offline` остановился на запрете sandbox создавать локальный HTTP-сервер: `cli_runs_sync_and_async_http_through_the_system_adapter`, PermissionDenied. Это не зелёный прогон.
- Повтор с доступом к локальным TCP-сокетам запущен; лог `/tmp/open-bsl-workspace-audit-network-1006.log`. Conformance-проверки завершились успешно: все 5 тестов, включая сверку всех оптимизирующих проходов, 188.33 с. Полный прогон остановился на новом HTML DOM string-output тесте: конструктор писателя пока ведёт к прежней заглушке, а обработчик ошибки вывел дополнительный отказ ОписаниеОшибки. Общий итог отрицательный.
- Запущен отдельный `cargo test --workspace --no-fail-fast --offline` с локальными сокетами для полного списка отказов: `/tmp/open-bsl-workspace-audit-all-failures-1006.log`. Завершился с тремя отказами: новый HTML DOM string-output; существующий information_register_factories ожидает отсутствие фабрики, добавленной для регистра накопления; существующий тест плотности конструкторов XML ожидает 1..=18 при добавленном коде 19. Ожидания не изменялись. Все остальные test-targets и doc-tests завершились без отказов. Это всё ещё отрицательный итог полного прогона, не основание для архивации.
- `cargo tree -p bsl-rt -e normal`: только прежний bsl-number и его числовые зависимости; манифест bsl-rt не изменён.
- `cargo tree -p bsl-vm -e normal`: bytecode/format/number/rt; ни SDK/СУБД, ни syntax/sema в normal-графе нет; манифест bsl-vm не изменён.

Архивация активного изменения и завершение всей задачи пока не обоснованы. Полнота оригинального startup, SDK-проекция бизнес-процессов, защищённые модули и все оставшиеся пункты tasks.md требуют отдельного завершения.

Сумма итогов 455 test-targets: 3203 passed, 3 failed, 1 ignored. Этот прогон предшествовал снятию запрета стандартной обработки после обработчика табличной части; последующее изменение проверено отдельным полным replay. Новый широкий зелёный gate всё ещё требуется после устранения перечисленных отказов.

## Повторная проверка ORM и сеансов после булева отказа

`cargo test --offline -p bsl-orm -p bsl-session --no-fail-fast` завершён: 45 итогов target, 391 passed, 1 failed, 0 ignored; лог `/tmp/open-bsl-orm-session-complete-1006.log`. Единственный отказ — существующий `factory_arity_is_checked_in_the_finite_receiver_context` в information_register_factories: прежнее ожидание отсутствия фабрики регистра накопления конфликтует с добавленным методом. Ожидание не изменено. Новый полный replay бизнес-процесса и остальные интеграционные проверки этих двух крейтов прошли. Этот результат не заменяет полный workspace gate.

Актуальный Cargo.lock использует open-sdbl/open-sdbl-db 0.9.8. В bsl-config-db приобретённые точки маршрута проецируются через with_business_process_route_points; собственного состава реквизитов/частей и номерных настроек бизнес-процесса в текущем SDK ещё нет. Новые нативные контрольные ресурсы и точный запрос на расширение SDK подготовлены отдельно; до приобретения этих сведений подставлять состав по именам в BSL нельзя.

## Проверка замкнутости фасада

После изменений обработчика проверки выполнен весь `cargo test --offline -p open-bsl-consumer`: 35 тестов прошли, 0 отказов; лог `/tmp/open-bsl-facade-consumer-1006.log`. Потребитель по-прежнему зависит только от open-bsl. Проверены библиотечные глобальные имена, данные хоста, экземпляры модулей и host-only вызов обработчика с возвратом изменённых параметров. Метаданные конфигурации остаются в bsl-orm; расширение runtime не требует зависимости фасада на ORM/SDK.

## Декомпилятор: проверка поддержанных форм

Повторный `cargo test --offline -p bsl-config-db native_image` завершён: 235 passed, 0 failed, 92 filtered; лог `/tmp/open-bsl-native-image-audit-1006.log`. Это фильтр тестов декодера, не весь крейт и не запуск защищённой конфигурации.

Зафиксированная ошибка оригинального образа UNF `Proc[25]/Cmd[4354]` возникает в recover_caught_return: принимается конкретная граница `handler + 9 == end` и проверенный хвост завершения. Общий восстановитель операторов уже обрабатывает другую измеренную форму try/catch; эти две формы не следует механически объединять без контроля возврата и стека. Следующий необходимый контроль должен различить возврат из тела/обработчика и продолжение после него, сохранив реальные инструкции компилятора и всю нативную квитанцию. Ни снятие проверки границ, ни пропуск метода не подтверждают восстановление оригинального модуля.

После поддержки нового защищённого контроля повторён compile-only UNF: 35.92 с, прежний Proc[25]/Cmd[4354], без исполнения BSL. Причина ограничивается не обычным try-return: в пропускаемом участке находится самостоятельный вход Proc[38] (flags 2, три параметра). Удаление участка нарушило бы сохранение объявленных входов. Полное восстановление оригинального образа остаётся неподтверждённым.

## Полный повтор после native try-return, flag-2 и Execute

- `cargo fmt --all -- --check`, `git diff --check`: успешно.
- `cargo clippy --offline --workspace --all-targets -- -D warnings`: успешно, 3.78 с; `/tmp/open-bsl-workspace-clippy-post-native-1006.log`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --no-deps`: успешно, 6.47 с; `/tmp/open-bsl-workspace-doc-post-native-1006.log`.
- `cargo test --offline --workspace --no-fail-fast` с доступом к локальным TCP-серверам: завершён, отрицательный итог. 455 итогов targets, 3217 passed, 3 failed, 1 ignored; `/tmp/open-bsl-workspace-tests-post-native-1006.log`. Все пять conformance-тестов прошли, включая сверку оптимизирующих проходов, 192.23 с.
- Три прежних отказа: bsl-cli html_dom string-output, bsl-session information_register_factories и bsl-xml constructor/function density. Новых отказов после изменений декомпилятора не найдено. Исходные регрессионные ожидания не изменены.
- `cargo tree --offline -p bsl-rt -e normal` и аналогичный bsl-vm: прежние normal-зависимости; файлы `/tmp/open-bsl-rt-tree-post-native-1006.txt` и `/tmp/open-bsl-vm-tree-post-native-1006.txt`.
- `openspec validate decompile-native-module-images --strict`: успешно.

Новый узкий bsl-config-db gate содержит 338 прошедших тестов, однако весь workspace остаётся красным. Ни закрытие полного плана, ни его архивация этим прогоном не обоснованы. Оригинальное выполнение demo/buh/unf и приобретение недостающих метаданных остаются отдельными незавершёнными требованиями.

## Полный повтор 2026-10-07 после общих тел и ссылочного слота

- `cargo test --offline --workspace --no-fail-fast` с доступом к локальным тестовым TCP-сокетам завершён с кодом 101: 455 итогов targets, 3223 passed, 3 failed, 1 ignored. Лог — `/tmp/open-bsl-workspace-tests-shared-bodies-1007.log`.
- Все пять conformance-тестов прошли, включая сверку оптимизирующих проходов и печатного байткода; 189.72 с.
- Три отказа прежние: `-p bsl-cli --test html_dom`, `-p bsl-session --test information_register_factories`, `-p bsl-xml --lib`. Существующие ожидания не изменялись. Новых отказов после шести добавленных тестов декодера нет.
- `cargo clippy --offline --workspace --all-targets -- -D warnings`: успешно, 5.59 с; `/tmp/open-bsl-workspace-clippy-shared-bodies-1007.log`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --no-deps`: успешно, 3.72 с; `/tmp/open-bsl-workspace-doc-shared-bodies-1007.log`.
- `cargo fmt --all -- --check`, `git diff --check`, проверка shell-синтаксиса платформенного runner: успешно.
- `openspec validate --all --strict`: 51 passed, 0 failed; `/tmp/open-bsl-openspec-shared-bodies-1007.log`.
- Normal-графы `bsl-rt` и `bsl-vm` побайтно совпадают с предыдущим снимком. Текущие файлы — `/tmp/open-bsl-rt-tree-shared-bodies-1007.txt` и `/tmp/open-bsl-vm-tree-shared-bodies-1007.txt`.

Декодер имеет 344 прошедших теста. Измерены и восстановлены узкие общие хвосты/тела и ссылочный параметр 5/0 частной процедуры с флагом 2. Необычные кадры оригинального защищённого образа остаются неподдержанными: успешного эталона внутреннего состояния параметра 1338 нет; частичные выводы после тайм-аутов не приняты как квитанции. Это не доказательство полноценного выполнения demo/buh/unf.

Общий итог workspace остаётся отрицательным. Согласованные изменения open-sdbl 0.9.8 завершены, но необходимые сведения о собственных реквизитах бизнес-процессов и регистрах накопления этим выпуском не приобретены. Полный план, исходные регрессионные ограничения и незавершённые требования сохраняются; активные изменения не архивированы.

## Полный прогон после включённого заполнения бизнес-процесса, 2026-10-07

Этот прогон включает вынесенный общий код заполнения атрибутов документов и бизнес-процессов, 21 строку нативного контроля включённых флагов, 16 строк контроля возвращённых параметров обработчика и отдельное воспроизведение ограничения статических корней. Он выполнен после этих изменений, а не перенесён из предыдущего запуска.

- `cargo test --workspace --offline --no-fail-fast` с разрешёнными локальными сокетами завершился с кодом 101: 456 итогов target, 3227 passed, 3 failed, 1 ignored. Лог: `/tmp/open-bsl-workspace-tests-business-filling-1007.log`.
- Все пять conformance-проверок прошли, включая оптимизирующие проходы и запуск напечатанного байткода. Проверки документов, новые контроли бизнес-процесса и `open-bsl-consumer` также прошли.
- Три отказа остались прежними: `bsl-cli --test html_dom`, `bsl-session --test information_register_factories`, `bsl-xml --lib`. Новых отказов после изменения общего заполнения этот прогон не обнаружил; старые ожидания не изменены.
- `cargo clippy --workspace --all-targets --offline -- -D warnings` прошёл, 20.88 с: `/tmp/open-bsl-workspace-clippy-business-filling-1007.log`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --offline` прошёл, 14.36 с: `/tmp/open-bsl-workspace-doc-business-filling-1007.log`.
- `openspec validate --all --strict`: 51 passed, 0 failed; `/tmp/open-bsl-openspec-business-filling-1007.log`.
- Normal-графы runtime и VM побайтно совпадают с предыдущим снимком: `/tmp/open-bsl-rt-tree-business-filling-1007.txt`, `/tmp/open-bsl-vm-tree-business-filling-1007.txt`.

Полный workspace gate остаётся отрицательным. Этот результат не подтверждает оригинальный startup demo/buh/unf, доступность объектных модулей вне статических корней или полноту SDK-метаданных; не даёт основания архивировать активные изменения либо завершать задачу.

## Workspace после подключения SDK 0.9.9, 2026-10-07

Подключены приобретённые метаданные бизнес-процессов и регистров накопления; исправлена проверка измеренных строковых квалификаторов в ORM. Все детали, собственные нативные контроли и новый результат compile-only demo приведены в [отчёте подключения SDK](sdk-099-runtime-metadata-integration.md).

`cargo test --workspace --offline --no-fail-fast` с разрешёнными локальными сокетами завершён: 456 итогов target, 3232 passed, 3 failed, 1 ignored, код 101. Лог `/tmp/open-bsl-sdk099-workspace-tests.log`. Все пять conformance-проверок, включая оптимизирующие проходы и напечатанный байткод, прошли. Отказы те же: html_dom, information_register_factories, bsl-xml --lib; старые тестовые ожидания не изменялись. Новые четыре теста проекции и сквозные нативные контроли прошли.

Workspace Clippy all-targets и rustdoc с запретом предупреждений прошли; fmt/diff-check и strict OpenSpec также. Normal-графы runtime/VM неизменны. SDK-зависимости в Cargo.lock — 0.9.9. Это не зелёный общий gate и не завершение оригинального startup; архивация активного изменения не выполнена.

## Workspace после декларации записи бизнес-процесса, 2026-10-07

Полный прогон: 457 итогов target, 3235 passed, 3 failed, 1 ignored; `/tmp/open-bsl-business-write-workspace-tests-1007.log`, код 101. Отказы прежние, существующие ожидания не изменены. Workspace Clippy, rustdoc, conformance, fmt/diff-check, API reference и strict OpenSpec прошли; normal-графы runtime/VM неизменны.

Добавлена декларация Write с явным отсутствием настоящего сохранения и сохранением ранее доступного экспорта. Реальные нативные записи выполнены только на собственной изолированной файловой ИБ. Оригинальный demo compile-only продвинулся до Прочитать. Подробнее — [отчёт записи](business-process-write-declaration.md). Общий gate отрицательный; startup и полный план не завершены.


## Чтение нового бизнес-процесса, 2026-10-07

После реализации `Read` прошли все 350 тестов `bsl-config-db` и три интеграционные проверки объявлений чтения/записи. Workspace Clippy с `-D warnings` прошёл за 3,83 с; rustdoc с `-D warnings` — за 12,40 с. Логи: `/tmp/open-bsl-business-read-workspace-clippy-1007.log`, `/tmp/open-bsl-business-read-doc-1007.log`. Strict OpenSpec — 51/51; normal-графы runtime/VM побайтно совпадают с предыдущим снимком. API перегенерирован.

Полный обычный sandbox-прогон `/tmp/open-bsl-business-read-workspace-tests-1007.log` встретил дополнительные `PermissionDenied` в сетевых/файловых тестах; он не является зелёным gate. Повтор CLI с разрешёнными локальными сокетами прошёл: `/tmp/open-bsl-business-read-cli-sockets-1007.log`. Начат отдельный полный повтор с разрешёнными локальными сокетами: `/tmp/open-bsl-business-read-workspace-sockets-1007.log`. Его итог на момент записи ещё не получен; завершение нужно проверить по живому process handle. Старые тестовые ожидания не менялись.

Compile-only demo теперь достигает `ВедущаяЗадача` после устранения прежнего отказа `Прочитать`: 3953 модуля, 4,59 с acquisition, 63,91 с полный проход. Этот результат не подтверждает исполнение startup.


Полный повтор с разрешёнными локальными сокетами завершён с кодом 101: 457 итогов target, 3237 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли. Отказы ровно прежние: `bsl-cli --test html_dom`, `bsl-session --test information_register_factories`, `bsl-xml --lib`. Новых отказов после реализации Read нет; исходные ожидания не менялись. Полный gate остаётся отрицательным, архивация и завершение плана не обоснованы. Лог `/tmp/open-bsl-business-read-workspace-sockets-1007.log`. Обычный sandbox-прогон тоже завершился отрицательно, с дополнительными отказами доступа; его результат не смешивается с этим повтором.


## Workspace после HeadTask, 2026-10-07

Полный прогон с разрешёнными локальными сокетами завершён с кодом 101: 458 итогов target, 3241 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли. Отказы ровно прежние: html_dom, information_register_factories, bsl-xml --lib. Старые ожидания не изменены. Лог `/tmp/open-bsl-head-task-workspace-tests-1007.log`. Workspace Clippy (3,80 с), rustdoc и strict OpenSpec прошли; API перегенерирован; normal-графы runtime/VM неизменны. Это прогон после HeadTask и перед реализацией Modified; он не подтверждает original startup и не разрешает архивацию полного плана.


## Workspace после Modified, 2026-10-07

Полный прогон завершён с кодом 101: 458 итогов target, 3242 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли. Отказы ровно прежние: html_dom, information_register_factories, bsl-xml --lib; старые ожидания не изменены. Лог `/tmp/open-bsl-business-modified-workspace-tests-1007.log`. Workspace Clippy — 10,76 с, rustdoc — 7,15 с; оба успешно. Strict OpenSpec — 51/51; fmt/diff/shell-проверки и сравнение normal-графов runtime/VM прошли. API перегенерирован. Реализация Modified имеет полный нативный replay 42 строк и не внесла новых отказов workspace. Полный gate остаётся отрицательным; original startup не проверен.


## Workspace после DataExchange, 2026-10-07

Полный прогон завершён с кодом 101: 458 итогов target, 3243 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли. Отказы прежние: html_dom, information_register_factories, bsl-xml --lib. Исходные ожидания не менялись. Лог `/tmp/open-bsl-business-data-exchange-workspace-tests-1007.log`. Workspace Clippy прошёл за 6,85 с, rustdoc — за 7,25 с; fmt/diff/shell-проверки и strict OpenSpec (51/51) прошли. Normal-графы RT/VM неизменны; API перегенерирован. Нативный replay DataExchange: 11 квалифицированных строк + 4 строки раздельной фазовой проверки. Полный gate остаётся отрицательным, original startup не подтверждён.


## Будущая ссылка бизнес-процесса, 2026-10-07

355 тестов адаптера БД и проверки совместимости экспортов прошли; нативный replay содержит 28 строк. Workspace Clippy — 11,01 с, rustdoc — 7,29 с, оба успешны. Fmt, diff, shell syntax и strict OpenSpec 51/51 прошли; normal-графы runtime/VM побайтно неизменны относительно DataExchange. API перегенерирован. Первоначальный workspace-прогон оборвался на conformance без итогового статуса и не засчитан. Повтор с терминальным exit-файлом запущен: `/tmp/open-bsl-business-new-reference-workspace-recheck-1007.log`; его результат ещё не зафиксирован.

Повтор после будущей ссылки завершился с кодом 101: 458 итогов, 3244 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли. Отказы ровно прежние: html_dom, information_register_factories, bsl-xml --lib. Исходные ожидания не менялись. Терминальный статус: `/tmp/open-bsl-business-new-reference-workspace-recheck-1007.exit`; лог с тем же префиксом. Это итог до объявления DeleteObjects; он не подтверждает последующие изменения.


## Объявление DeleteObjects, 2026-10-07

После OpenSpec-добавления и красного воспроизведения UndefinedFunction все 133 теста ORM прошли. Новые проверки покрывают catchable read-only rejection, сохранение массива/таблицы и арности 1…6 с отклонением 0/7. Реальное удаление не реализовано, нативный пустой вызов не подменён успешной заглушкой. API перегенерирован; Clippy 10,76 с, rustdoc 7,22 с, strict OpenSpec 51/51, fmt/diff и consumer check прошли. Normal-графы runtime/VM неизменны. Workspace `/tmp/open-bsl-delete-objects-workspace-1007.log` ещё не терминален. Compile-only demo теперь останавливается на ГрафическаяСхема; это не выполнение оригинального модуля сеанса.

Полный прогон после объявления DeleteObjects завершился с кодом 101: 458 итогов, 3246 passed, 3 failed, 1 ignored. Все пять conformance-проверок прошли; отказы ровно прежние три, исходные ожидания не менялись. Терминальный exit-файл: `/tmp/open-bsl-delete-objects-workspace-1007.exit`. Новых отказов после объявления не обнаружено; полный gate остаётся отрицательным.


## Пустая GraphicalSchema, 2026-10-07

Три новых facade-теста прошли: 11 нативных строк начального состояния, 6 нормализованных строк результатов неудачного поиска и арность конструктора. Сохранены полные русские и английские receipts, а также 9 строк нативных ошибок Find. Из 46 unit-тестов bsl-spreadsheet 45 прошли; единственный отказ — прежнее ожидание кодов [1, 2] при добавленном коде 3. Разрешение на изменение ожидания запрошено, старый тест не изменён. Workspace Clippy и rustdoc прошли; справочник API перегенерирован. Полный workspace после добавления конструктора не засчитан, предыдущий прогон относится к состоянию до GraphicalSchema. Compile-only demo дошёл до ПоложениеКартинкиЭлементаГрафическойСхемы, прикладной код не исполнялся.


## Перечисление GraphicalSchemaItemPictureLocation, 2026-10-07

Полный нативный receipt 14 строк совпал; дополнительная проверка folded lookup и неизвестного члена прошла. Runtime unit — 307 passed, facade graphics — 5 passed. API обновлён; strict OpenSpec 51/51, fmt/diff и сравнение normal-графов runtime/VM прошли. Workspace `/tmp/open-bsl-graphical-picture-location-workspace-1007.log` запущен с отдельным терминальным exit-файлом. Прежние три отказа и новое неизменённое ожидание кодов bsl-spreadsheet не выдаются за зелёный gate.

Clippy workspace завершился успешно за 51,24 с, rustdoc — за 42,57 с. Это времена отдельных проверок с общим build lock, не сравнительный benchmark. Compile-only прошёл прежний отказ и дошёл до ФигурыГрафическойСхемы. Workspace-прогон ещё не терминален; общий gate не закрыт.

Полный workspace после picture location завершился с кодом 101: 459 итогов, 3250 passed, 4 failed, 1 ignored. Все пять conformance прошли. Отказы: прежние html_dom, information_register_factories, bsl-xml --lib и неизменённый тест плотности кодов bsl-spreadsheet. Старые ожидания не исправлялись. Терминальный статус `/tmp/open-bsl-graphical-picture-location-workspace-1007.exit`. Эта проверка закончилась до реализации фигур/линий/стрелок.


## GraphicalSchemaShapes и line/arrow enums, 2026-10-07

До изменения runtime закончился предыдущий полный прогон. После OpenSpec/красных воспроизведений реализованы четыре полных набора через прежние таблицы; все 66 нативных строк совпали. Runtime unit — 307 passed, facade graphics — 8 passed. API обновлён; normal-графы runtime/VM неизменны. Новый workspace `/tmp/open-bsl-graphical-shapes-lines-workspace-1007.log` запущен; старое ожидание диапазона конструкторов не исправлялось, предыдущий полный итог к этому снимку не переносится.

Clippy workspace 48,18 с, rustdoc 39,41 с завершились успешно; explicit consumer check прошёл. Strict OpenSpec — 51/51, fmt/diff и normal-графы неизменны. Compile-only demo дошёл до Предопределенный в модуле объекта плана видов характеристик. Полный workspace после фигур/линий ещё не терминален; его результат не предполагается по предыдущему прогону.

Полный workspace после фигур/линий завершился с кодом 101: 459 итогов, 3253 passed, 4 failed, 1 ignored. Все пять conformance прошли. Отказы те же четыре: html_dom, information_register_factories, bsl-spreadsheet --lib (диапазон кодов), bsl-xml --lib. Старые ожидания не менялись. Терминальный статус `/tmp/open-bsl-graphical-shapes-lines-workspace-1007.exit`. Проверка закончилась до реализации свойства характеристик.


## Characteristic Predefined, 2026-10-07

Нативный контроль 9 строк совпал через SessionFactory после красного UndefinedVariable. Восемь прежних characteristic_objects тестов прошли без изменений. API перегенерирован; normal-графы runtime/VM побайтно прежние. Workspace `/tmp/open-bsl-characteristic-predefined-workspace-1007.log` запущен с отдельным exit-файлом; результаты предыдущего прогона относятся к снимку до getter. Неизменённые старые ожидания не скрываются и полный gate пока не закрыт.

Clippy 11,47 с, rustdoc 7,75 с, strict OpenSpec 51/51, fmt/diff/shell syntax и consumer check прошли. Compile-only demo прошёл ВидыДоступа и остановился на Получить в менеджере регистра сведений. Workspace-прогон после getter ещё не терминален; состояние старых ожиданий неизменно.

Полный workspace после Predefined завершился с кодом 101: 460 итогов, 3254 passed, 4 failed, 1 ignored. Отказы прежние: html_dom, information_register_factories, bsl-spreadsheet --lib (диапазон кодов), bsl-xml --lib. Старые ожидания не менялись. Терминальный статус `/tmp/open-bsl-characteristic-predefined-workspace-1007.exit`; полный gate остаётся отрицательным.

## Начальная реализация InformationRegister.Get, 2026-10-07

После 47 строк нативных контролей и strict OpenSpec добавлены явные
настройки независимого регистра в ORM. Метод строит BSL-запрос через
существующий адаптер текущего пользователя, приводит числовой отбор
по объявленному типу и сохраняет точную дату периодического чтения.
Поддержана измеренная форма Number(5,0)/Number(15,0), Nonperiodic/Day;
неизвестные настройки и другие типы отвергаются явно. SDK-проекция
ещё отсутствует, оригинальный startup не подтверждён.

Красное воспроизведение: `/tmp/open-bsl-information-get-red-1007.log`.
Пять новых проверок прошли; весь ORM — 138 passed, 0 failed.
Одноимённый экспорт менеджера сохраняет собственный результат и арность.
Clippy workspace — 3,97 с, rustdoc — 14,48 с; оба прошли.
Consumer check, fmt/diff, strict OpenSpec прошли; API перегенерирован,
normal-графы RT/VM побайтно прежние. Полный workspace запущен в
`/tmp/open-bsl-information-get-workspace-1007.log` с отдельным exit-файлом;
его итог ещё не получен. Старые ожидания не менялись.

Две новые SessionFactory-проверки подтвердили голый Получить/gEt,
сохранение одноимённой модульной переменной и конечную арность.
Они выполнены отдельно после запуска полного workspace и не добавляются
задним числом к его итогам. Лог `/tmp/open-bsl-information-get-session-1007.log`.
Compile-only demo прошёл прежний отказ: 3953 модуля, acquisition 4,58 с,
полный проход 89,34 с. Следующий отказ — UndefinedFunction("Установить")
в Константа.ИспользоватьОтметкиВремени.МодульМенеджера. Исходный вызов
проверен read-only: ОбновитьЗначение условно записывает константу после
чтения текущего значения. Поэтому успешная пустая реализация Set недопустима.
Это не исполнение startup.

Полный workspace после начального Get завершился с кодом 101: 461 итогов, 3259 passed, 4 failed, 1 ignored. Все пять conformance прошли. Отказы те же четыре: html_dom, information_register_factories, bsl-spreadsheet --lib (неизменённый диапазон кодов), bsl-xml --lib. Терминальный статус `/tmp/open-bsl-information-get-workspace-1007.exit`. Две новые SessionFactory-проверки прошли отдельно; полный gate остаётся отрицательным.

## Опциональная запись константы, 2026-10-07

Сначала strict OpenSpec и красный тест отсутствующего метода, затем
Set/Установить добавлен как делегирование DataAccess::set_constant_value.
Значение, метаданные и текущий principal передаются адаптеру; default
явно отвергает запись. DatabaseDataAccess не стал SQL-писателем, не
подменяет Get локальным кешем. Пять новых ORM/SessionFactory проверок
прошли: успешное делегирование тестовому писателю, отказ, неизменённое
чтение, изоляция, арность, алиасы, голый вызов и одноимённый экспорт.
Нативное хранение, приведение, права записи и обработчики ещё не реализованы.

Сигнатура проверена полным installed help 8.3.27.2342:
`platform-syntax-help:syntax-context:objects/catalog125/catalog136/object211/methods/Set548.html`.
Она не использована как доказательство семантики преобразования значений.
API перегенерирован. Clippy workspace 3,81 с, rustdoc 10,38 с, отдельный
Clippy нового SessionFactory-теста, fmt/diff, consumer check и неизменность
normal-графов RT/VM подтверждены. Workspace запущен в
`/tmp/open-bsl-constant-set-workspace-1007.log` с отдельным exit-файлом;
его результат ещё не получен. Проверка compile-only также запущена отдельно.
Старые тестовые ожидания не менялись.

Compile-only после Set завершился с отказом на МоментВремени в
Документ.ПередачаДелВАрхив.МодульОбъекта: 3953 модуля, acquisition
4,62 с, полный проход 89,39 с. Прикладной BSL не выполнялся.
Существующее намеренное ограничение конструктора изучено; сохранён
отдельный нативный контроль нового незаписанного документа (10 строк)
в `fixtures/document-point-in-time/`. Тип и сравнение пока не реализованы.

Отдельная новая проверка Get подтвердила сохранение отказа адаптера
и отсутствие выдачи результата другого сеанса. Шесть focused Get-тестов
прошли; это не доказательство реальных SDK прав/RLS. Полный workspace,
запущенный ранее, не включает этот добавленный после запуска тест.

Полный workspace после Set завершился с кодом 101: 464 итогов, 3266 passed, 4 failed, 1 ignored. Все пять conformance прошли. Отказы прежние: html_dom, information_register_factories, bsl-spreadsheet --lib (неизменённый диапазон кодов), bsl-xml --lib. Терминальный статус `/tmp/open-bsl-constant-set-workspace-1007.exit`. Добавленный после запуска тест отказа Get выполнен отдельно; он не включён в эти итоги. Полный gate остаётся отрицательным.

Десять новых нативных сравнений групп GUID подтвердили физический
порядок существующего uuid::to_1c_bytes. Новый точечный runtime-тест
сравнил все десять строк и прошёл; его Clippy тоже прошёл. Это проверка
общего механизма UUID, а не завершённая реализация PointInTime.
Всего в собственных квитанциях момента времени теперь 51 строка;
хеши артефактов проверены. Порядок разных видов ссылок ещё открыт.
После последнего полного workspace изменились только новые тесты,
замеры и документация; runtime-код не менялся.

Сравнение трёх видов документов дало 27 строк, неизменных при перестановке
UUID; нативная структура хранения Document53/55/56 совпала с порядком
видов. Исправленная квитанция содержит 53 строки, первоначальный отказ
диагностики колонок сохранён отдельно. Имена колонок фактически английские.
Это не доказательство порядка всех классов метаданных.
Read-only оригинальный модуль передаёт исключающую границу момента в запрос.
SDK 0.9.9 синтезирует поле PointInTime, но ParameterValue не представляет
момент или границу. Подготовлен отдельный запрос
`docs/plans/open-sdbl-temporal-query-parameters-request.md`; версия не назначена,
новый агент не запускался. Хеши собственных артефактов, strict OpenSpec,
shell syntax и diff check прошли. Runtime-код в этом шаге не менялся.
