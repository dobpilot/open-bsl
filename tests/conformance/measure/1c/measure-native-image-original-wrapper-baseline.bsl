Попытка
    ОбъектКадра = Обработки.Замеры.Создать();
    АргументКадра0 = 5;
    АргументКадра1 = 6;
    АргументКадра2 = 9;
    РезультатКадра = ОбъектКадра.__ПроверитьОригинальныйКадр(АргументКадра0, АргументКадра1, АргументКадра2);
    Сообщить("image.original-wrapper-baseline.value=" + Строка(РезультатКадра));
    Сообщить("image.original-wrapper-baseline.argument0=" + Строка(АргументКадра0));
    Сообщить("image.original-wrapper-baseline.argument1=" + Строка(АргументКадра1));
    Сообщить("image.original-wrapper-baseline.argument2=" + Строка(АргументКадра2));
Исключение
    Сообщить("image.original-wrapper-baseline.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.original-wrapper-baseline.done=1");
