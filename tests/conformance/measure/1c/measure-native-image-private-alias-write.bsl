Попытка
    АргументЗаписи = 5;
    РезультатЗаписи = ЗамерыТранзитивногоРодителя.Проба(АргументЗаписи);
    Сообщить("image.private-alias-write.call1.type=" + Строка(ТипЗнч(РезультатЗаписи)));
    Сообщить("image.private-alias-write.call1.value=" + Строка(РезультатЗаписи));
    Сообщить("image.private-alias-write.call1.argument=" + Строка(АргументЗаписи));
Исключение
    Сообщить("image.private-alias-write.call1.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументЗаписи = 5;
    РезультатЗаписи = ЗамерыТранзитивногоРодителя.Проба(АргументЗаписи);
    Сообщить("image.private-alias-write.call2.type=" + Строка(ТипЗнч(РезультатЗаписи)));
    Сообщить("image.private-alias-write.call2.value=" + Строка(РезультатЗаписи));
    Сообщить("image.private-alias-write.call2.argument=" + Строка(АргументЗаписи));
Исключение
    Сообщить("image.private-alias-write.call2.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-alias-write.done=1");
