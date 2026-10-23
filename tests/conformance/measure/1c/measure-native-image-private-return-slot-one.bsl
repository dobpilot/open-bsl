Попытка
    АргументВозврата = 5;
    ЗначениеВозврата = ЗамерыТранзитивногоРодителя.Проба(АргументВозврата);
    Сообщить("image.private-return-slot-one.type=" + Строка(ТипЗнч(ЗначениеВозврата)));
    Сообщить("image.private-return-slot-one.value=" + Строка(ЗначениеВозврата));
    Сообщить("image.private-return-slot-one.argument=" + Строка(АргументВозврата));
Исключение
    Сообщить("image.private-return-slot-one.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-return-slot-one.done=1");
