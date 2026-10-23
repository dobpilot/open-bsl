Попытка
    Сообщить("graphical.picture.top.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы.Верх));
    Сообщить("graphical.picture.top.alias" + Символ(9) + ?(ПоложениеКартинкиЭлементаГрафическойСхемы.Верх = GraphicalSchemaItemPictureLocation.Top, "1", "0"));
    Сообщить("graphical.picture.left.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы.Лево));
    Сообщить("graphical.picture.left.alias" + Символ(9) + ?(ПоложениеКартинкиЭлементаГрафическойСхемы.Лево = GraphicalSchemaItemPictureLocation.Left, "1", "0"));
    Сообщить("graphical.picture.bottom.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы.Низ));
    Сообщить("graphical.picture.bottom.alias" + Символ(9) + ?(ПоложениеКартинкиЭлементаГрафическойСхемы.Низ = GraphicalSchemaItemPictureLocation.Bottom, "1", "0"));
    Сообщить("graphical.picture.right.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы.Право));
    Сообщить("graphical.picture.right.alias" + Символ(9) + ?(ПоложениеКартинкиЭлементаГрафическойСхемы.Право = GraphicalSchemaItemPictureLocation.Right, "1", "0"));
    Сообщить("graphical.picture.center.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы.Центр));
    Сообщить("graphical.picture.center.alias" + Символ(9) + ?(ПоложениеКартинкиЭлементаГрафическойСхемы.Центр = GraphicalSchemaItemPictureLocation.Center, "1", "0"));
    Сообщить("graphical.picture.type" + Символ(9) + Строка(ТипЗнч(ПоложениеКартинкиЭлементаГрафическойСхемы.Верх)));
    Сообщить("graphical.picture.type.alias" + Символ(9) + ?(Тип("ПоложениеКартинкиЭлементаГрафическойСхемы") = Тип("GraphicalSchemaItemPictureLocation"), "1", "0"));
    Сообщить("graphical.picture.global.text" + Символ(9) + Строка(ПоложениеКартинкиЭлементаГрафическойСхемы));
    Сообщить("graphical.picture.global.type" + Символ(9) + Строка(ТипЗнч(ПоложениеКартинкиЭлементаГрафическойСхемы)));
Исключение
    Сообщить("graphical.picture.error" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
