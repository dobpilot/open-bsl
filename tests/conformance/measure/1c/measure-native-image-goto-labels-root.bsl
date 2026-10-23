Попытка
    Сообщить("image.native.goto_labels.root.first=" + Строка(ЗамерыТранзитивногоРодителя.ПерваяПробы()));
Исключение
    Сообщить("image.native.goto_labels.root.first_error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    Сообщить("image.native.goto_labels.root.second=" + Строка(ЗамерыТранзитивногоРодителя.ВтораяПробы()));
Исключение
    Сообщить("image.native.goto_labels.root.second_error=" + ОписаниеОшибки());
КонецПопытки;
