// Нативная проба ветвления с циклом и общим продолжением.
Попытка
    СчетчикПробы = 3;
    Сообщить("image.native.branch_loop.iterate.result=" + Строка(ЗамерыТранзитивногоРодителя.Проба(Истина, СчетчикПробы)));
    Сообщить("image.native.branch_loop.iterate.argument=" + Строка(СчетчикПробы));
    СчетчикПробы = 0;
    Сообщить("image.native.branch_loop.empty.result=" + Строка(ЗамерыТранзитивногоРодителя.Проба(Истина, СчетчикПробы)));
    Сообщить("image.native.branch_loop.empty.argument=" + Строка(СчетчикПробы));
    СчетчикПробы = 3;
    Сообщить("image.native.branch_loop.skip.result=" + Строка(ЗамерыТранзитивногоРодителя.Проба(Ложь, СчетчикПробы)));
    Сообщить("image.native.branch_loop.skip.argument=" + Строка(СчетчикПробы));
Исключение
    Сообщить("image.native.branch_loop.failure=" + ОписаниеОшибки());
КонецПопытки;
