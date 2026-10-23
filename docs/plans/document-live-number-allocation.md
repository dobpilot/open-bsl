# Реальное выделение номеров документов

Статус: отложено пользователем 21 октября 2026 года, исключено из текущего
goal инициализации сеанса. Это не реализованная возможность и не критерий
завершения текущей задачи.

Существующие обработчик ПриУстановкеНовогоНомера, передача параметров,
приобретённые настройки области нумерации и явный отказ неподдержанного
адаптера сохраняются. Локальный счётчик или тестовый адаптер не заменяют
выделение номеров в реальной информационной базе. Нормативные требования
к сессионному адаптеру не изменяются; поддержка реальной базы остаётся
отдельной будущей реализацией. Перед её возобновлением требуется OpenSpec
для конкретного поддерживаемого контура, а не попутное расширение startup.

Сохранённый исходный объём из add-document-object-instances:

- [ ] 39. Measure number-assignment event cancellation and prefix mutation, obtain the required original numbering metadata, and implement allocation against the actual infobase without substituting an object-local counter; replay native behavior and repeat original startup.

- [ ] 51. Use the verified numbering scope and stored-number measurements to design and implement actual session allocation, including required access rules, without treating metadata projection or a test adapter as completed live numbering.

Нативные сценарии отмены стандартной обработки, изменения префикса,
периодичности, нумератора, сохранённых номеров и прав не удаляются. Они
остаются входными свидетельствами для будущей реализации; зелёный запуск
модуля сеанса не подменяет проверку реального выделения номеров.
