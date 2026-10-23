Попытка
    АргументВозврата = 5;
    ЗначениеВозврата = ЗамерыТранзитивногоРодителя.Проба(АргументВозврата);
    Сообщить("image.private-return-baseline.type=" + Строка(ТипЗнч(ЗначениеВозврата)));
    Сообщить("image.private-return-baseline.value=" + Строка(ЗначениеВозврата));
    Сообщить("image.private-return-baseline.argument=" + Строка(АргументВозврата));
Исключение
    Сообщить("image.private-return-baseline.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-return-baseline.done=1");
