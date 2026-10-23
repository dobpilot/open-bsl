Попытка
    Сообщить("image.native.unreachable_cycle.true=" + Строка(ЗамерыТранзитивногоРодителя.ИстиннаяПробы()));
    Сообщить("image.native.unreachable_cycle.false=" + Строка(ЗамерыТранзитивногоРодителя.ЛожнаяПробы()));
    Сообщить("image.native.unreachable_cycle.next=" + Строка(ЗамерыТранзитивногоРодителя.СледующаяПробы()));
    Сообщить("image.native.unreachable_cycle.jump=" + Строка(ЗамерыТранзитивногоРодителя.ПропускПробы()));
Исключение
    Сообщить("image.native.unreachable_cycle.failure=" + ОписаниеОшибки());
КонецПопытки;
