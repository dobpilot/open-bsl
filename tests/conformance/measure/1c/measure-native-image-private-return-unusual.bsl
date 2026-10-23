Попытка
    АргументВозврата = 5;
    ЗначениеВозврата = ЗамерыТранзитивногоРодителя.Проба(АргументВозврата);
    Сообщить("image.private-return-unusual.type=" + Строка(ТипЗнч(ЗначениеВозврата)));
    Сообщить("image.private-return-unusual.value=" + Строка(ЗначениеВозврата));
    Сообщить("image.private-return-unusual.argument=" + Строка(АргументВозврата));
Исключение
    Сообщить("image.private-return-unusual.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-return-unusual.done=1");
