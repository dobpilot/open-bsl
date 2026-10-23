// Перечисления, обнаруженные после конструктора фиксированной структуры.

Сообщить("ColorDepth.ROOT" + Символ(9) + Строка(ГлубинаЦвета));
Сообщить("ColorDepth.TYPE" + Символ(9) + Строка(ТипЗнч(ГлубинаЦвета.БитНаПиксел1)));
Сообщить("ColorDepth.BitPerPixel1" + Символ(9) + Строка(ГлубинаЦвета.БитНаПиксел1));
Сообщить("ColorDepth.BitPerPixel24" + Символ(9) + Строка(ГлубинаЦвета.БитНаПиксел24));
Сообщить("ColorDepth.BitPerPixel32" + Символ(9) + Строка(ГлубинаЦвета.БитНаПиксел32));
Сообщить("ColorDepth.BitPerPixel4" + Символ(9) + Строка(ГлубинаЦвета.БитНаПиксел4));
Сообщить("ColorDepth.BitPerPixel8" + Символ(9) + Строка(ГлубинаЦвета.БитНаПиксел8));
Если ГлубинаЦвета.БитНаПиксел1 = ColorDepth.BitPerPixel1
    И ГлубинаЦвета.БитНаПиксел24 = ColorDepth.BitPerPixel24
    И ГлубинаЦвета.БитНаПиксел32 = ColorDepth.BitPerPixel32
    И ГлубинаЦвета.БитНаПиксел4 = ColorDepth.BitPerPixel4
    И ГлубинаЦвета.БитНаПиксел8 = ColorDepth.BitPerPixel8 Тогда
    Сообщить("ColorDepth.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ColorDepth.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ToolTipRepresentation.ROOT" + Символ(9) + Строка(ОтображениеПодсказки));
Сообщить("ToolTipRepresentation.TYPE" + Символ(9) + Строка(ТипЗнч(ОтображениеПодсказки.Авто)));
Сообщить("ToolTipRepresentation.Auto" + Символ(9) + Строка(ОтображениеПодсказки.Авто));
Сообщить("ToolTipRepresentation.Balloon" + Символ(9) + Строка(ОтображениеПодсказки.Всплывающая));
Сообщить("ToolTipRepresentation.Button" + Символ(9) + Строка(ОтображениеПодсказки.Кнопка));
Сообщить("ToolTipRepresentation.None" + Символ(9) + Строка(ОтображениеПодсказки.Нет));
Сообщить("ToolTipRepresentation.ShowAuto" + Символ(9) + Строка(ОтображениеПодсказки.ОтображатьАвто));
Сообщить("ToolTipRepresentation.ShowTop" + Символ(9) + Строка(ОтображениеПодсказки.ОтображатьСверху));
Сообщить("ToolTipRepresentation.ShowLeft" + Символ(9) + Строка(ОтображениеПодсказки.ОтображатьСлева));
Сообщить("ToolTipRepresentation.ShowBottom" + Символ(9) + Строка(ОтображениеПодсказки.ОтображатьСнизу));
Сообщить("ToolTipRepresentation.ShowRight" + Символ(9) + Строка(ОтображениеПодсказки.ОтображатьСправа));
Если ОтображениеПодсказки.Авто = ToolTipRepresentation.Auto
    И ОтображениеПодсказки.Всплывающая = ToolTipRepresentation.Balloon
    И ОтображениеПодсказки.Кнопка = ToolTipRepresentation.Button
    И ОтображениеПодсказки.Нет = ToolTipRepresentation.None
    И ОтображениеПодсказки.ОтображатьАвто = ToolTipRepresentation.ShowAuto
    И ОтображениеПодсказки.ОтображатьСверху = ToolTipRepresentation.ShowTop
    И ОтображениеПодсказки.ОтображатьСлева = ToolTipRepresentation.ShowLeft
    И ОтображениеПодсказки.ОтображатьСнизу = ToolTipRepresentation.ShowBottom
    И ОтображениеПодсказки.ОтображатьСправа = ToolTipRepresentation.ShowRight Тогда
    Сообщить("ToolTipRepresentation.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ToolTipRepresentation.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ClientApplicationType.ROOT" + Символ(9) + Строка(ТипКлиентскогоПриложения));
Сообщить("ClientApplicationType.TYPE" + Символ(9) + Строка(ТипЗнч(ТипКлиентскогоПриложения.ВебКлиент)));
Сообщить("ClientApplicationType.WebClient" + Символ(9) + Строка(ТипКлиентскогоПриложения.ВебКлиент));
Сообщить("ClientApplicationType.ExternalConnection" + Символ(9) + Строка(ТипКлиентскогоПриложения.ВнешнееСоединение));
Сообщить("ClientApplicationType.MobileAppClient" + Символ(9) + Строка(ТипКлиентскогоПриложения.МобильноеПриложениеКлиент));
Сообщить("ClientApplicationType.MobileClient" + Символ(9) + Строка(ТипКлиентскогоПриложения.МобильныйКлиент));
Сообщить("ClientApplicationType.ThickClient" + Символ(9) + Строка(ТипКлиентскогоПриложения.ТолстыйКлиент));
Сообщить("ClientApplicationType.ThinClient" + Символ(9) + Строка(ТипКлиентскогоПриложения.ТонкийКлиент));
Если ТипКлиентскогоПриложения.ВебКлиент = ClientApplicationType.WebClient
    И ТипКлиентскогоПриложения.ВнешнееСоединение = ClientApplicationType.ExternalConnection
    И ТипКлиентскогоПриложения.МобильноеПриложениеКлиент = ClientApplicationType.MobileAppClient
    И ТипКлиентскогоПриложения.МобильныйКлиент = ClientApplicationType.MobileClient
    И ТипКлиентскогоПриложения.ТолстыйКлиент = ClientApplicationType.ThickClient
    И ТипКлиентскогоПриложения.ТонкийКлиент = ClientApplicationType.ThinClient Тогда
    Сообщить("ClientApplicationType.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ClientApplicationType.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

