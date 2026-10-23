Попытка
    АргументВозврата = 5;
    ЗначениеВозврата = ЗамерыТранзитивногоРодителя.Проба(АргументВозврата);
    Сообщить("image.private-return-constant.type=" + Строка(ТипЗнч(ЗначениеВозврата)));
    Сообщить("image.private-return-constant.value=" + Строка(ЗначениеВозврата));
    Сообщить("image.private-return-constant.argument=" + Строка(АргументВозврата));
Исключение
    Сообщить("image.private-return-constant.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-return-constant.done=1");
