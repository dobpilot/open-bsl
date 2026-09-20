# Передача временного файла из worker

## Причина и границы

Измеренный CreateTempFileAsync должен возвращать обещание файлового потока.
Существующие FileHandle и TemporaryFileResource могут содержать Rc;
FileSystem::background_access разрешает доступ к сервису из другого потока,
но не обещает переносимости его результатов. Простая отправка прежнего
OpenedTemporaryFile через канал невозможна без изменения старого host API.

Добавлены TransferableTemporaryFile и отдельная необязательная операция
FileSystem::create_transferable_temporary_file. Оба дескриптора нового
результата требуют Send; локальные трейты и реестр не меняют ограничения.
Конструктор требует право очистки, пути без ресурса недостаточно.
Преобразование into_local использует прежний OpenedTemporaryFile::new_owned,
регистрация и отказ закрытого реестра остаются единственной реализацией.
Нет повторного открытия, файлового I/O при передаче и удаления при Drop.

Это контракт embedding-host, не утверждение о внутреннем устройстве 1С.
Нативная база, oracle и маркеры неизмеренных вопросов не изменялись.
Нет новых зависимостей, изменений BSL-интерфейса или формата байткода.

Сам CreateTempFileAsync ещё не подключён. Системное право очистки,
неблокирующее ожидание и ответственность за поздний результат остаются
обязательными задачами. Пользователю задан вопрос, допустимо ли ожидание
незавершённого создания при закрытии State ради существующего локального
callback. Без ответа блокирующее закрытие не вводилось.

## Проверки

Первоначальные тесты не собирались из-за отсутствия нового типа и метода:
`/tmp/open-bsl-temp-transferable-red.log`. После реализации три теста
temporary_file_transfer проходят через background_access и настоящий
worker: передача исходного носителя с данными и позицией, регистрация права,
отказ закрытого реестра с сохранением результата, передача другому сеансу,
освобождение обеих форм без удаления. Фальшивый host запрещает повторные
открытия и обращения по пути; счётчики подтверждают отсутствие I/O
при преобразовании и регистрации.

Дополнена проверка default Unsupported в temporary_directory. Прежние пять
тестов temporary_file_ownership продолжают использовать непереносимые Rc
и проходят без изменения. Фокусный лог:
`/tmp/open-bsl-temp-transferable-focused.log`.

Полный `cargo test --workspace` завершился с кодом 0, включая conformance,
plain/optimize, проверку API reference, ZIP/stream и фоновые задания.
Успешны `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`,
`git diff --check` и строгая валидация всех 11 пунктов OpenSpec.
API reference регенерирован штатной командой. Логи:
`/tmp/open-bsl-temp-transferable-workspace.log`,
`/tmp/open-bsl-temp-transferable-clippy.log`,
`/tmp/open-bsl-temp-transferable-doc.log`,
`/tmp/open-bsl-temp-transferable-api.log`.
