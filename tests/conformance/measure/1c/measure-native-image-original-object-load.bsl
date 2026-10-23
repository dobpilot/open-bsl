Попытка
    Сообщить("image.original-object-load.enter=1");
    ОбъектИсходногоОбраза = Обработки.Замеры.Создать();
    Сообщить("image.original-object-load.created=1");
Исключение
    Сообщить("image.original-object-load.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.original-object-load.done=1");
