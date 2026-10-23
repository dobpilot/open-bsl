Сообщить("image.original-private37-call.before=1");
Попытка
    ОбъектКадра = Обработки.Замеры.Создать();
    АргументКадра0 = 5;
    АргументКадра1 = 6;
    АргументКадра2 = 9;
    РезультатКадра = ОбъектКадра.__ПроверитьОригинальныйКадр(АргументКадра0, АргументКадра1, АргументКадра2);
    Сообщить("image.original-private37-call.value=" + Строка(РезультатКадра));
    Сообщить("image.original-private37-call.argument0=" + Строка(АргументКадра0));
    Сообщить("image.original-private37-call.argument1=" + Строка(АргументКадра1));
    Сообщить("image.original-private37-call.argument2=" + Строка(АргументКадра2));
Исключение
    Сообщить("image.original-private37-call.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.original-private37-call.done=1");
