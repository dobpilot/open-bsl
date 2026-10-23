// Перечисления, обнаруженные при компиляции УНФ.

Сообщить("FormDecorationType.ROOT" + Символ(9) + Строка(ВидДекорацииФормы));
Сообщить("FormDecorationType.TYPE" + Символ(9) + Строка(ТипЗнч(ВидДекорацииФормы.Картинка)));
Сообщить("FormDecorationType.Picture" + Символ(9) + Строка(ВидДекорацииФормы.Картинка));
Сообщить("FormDecorationType.Label" + Символ(9) + Строка(ВидДекорацииФормы.Надпись));
Если ВидДекорацииФормы.Картинка = FormDecorationType.Picture
    И ВидДекорацииФормы.Надпись = FormDecorationType.Label Тогда
    Сообщить("FormDecorationType.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("FormDecorationType.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("RadioButtonType.ROOT" + Символ(9) + Строка(ВидПереключателя));
Сообщить("RadioButtonType.TYPE" + Символ(9) + Строка(ТипЗнч(ВидПереключателя.Авто)));
Сообщить("RadioButtonType.Auto" + Символ(9) + Строка(ВидПереключателя.Авто));
Сообщить("RadioButtonType.RadioButton" + Символ(9) + Строка(ВидПереключателя.Переключатель));
Сообщить("RadioButtonType.Tumbler" + Символ(9) + Строка(ВидПереключателя.Тумблер));
Если ВидПереключателя.Авто = RadioButtonType.Auto
    И ВидПереключателя.Переключатель = RadioButtonType.RadioButton
    И ВидПереключателя.Тумблер = RadioButtonType.Tumbler Тогда
    Сообщить("RadioButtonType.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("RadioButtonType.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ChildFormItemsGroup.ROOT" + Символ(9) + Строка(ГруппировкаПодчиненныхЭлементовФормы));
Сообщить("ChildFormItemsGroup.TYPE" + Символ(9) + Строка(ТипЗнч(ГруппировкаПодчиненныхЭлементовФормы.Вертикальная)));
Сообщить("ChildFormItemsGroup.Vertical" + Символ(9) + Строка(ГруппировкаПодчиненныхЭлементовФормы.Вертикальная));
Сообщить("ChildFormItemsGroup.Horizontal" + Символ(9) + Строка(ГруппировкаПодчиненныхЭлементовФормы.Горизонтальная));
Сообщить("ChildFormItemsGroup.AlwaysHorizontal" + Символ(9) + Строка(ГруппировкаПодчиненныхЭлементовФормы.ГоризонтальнаяВсегда));
Сообщить("ChildFormItemsGroup.HorizontalIfPossible" + Символ(9) + Строка(ГруппировкаПодчиненныхЭлементовФормы.ГоризонтальнаяЕслиВозможно));
Если ГруппировкаПодчиненныхЭлементовФормы.Вертикальная = ChildFormItemsGroup.Vertical
    И ГруппировкаПодчиненныхЭлементовФормы.Горизонтальная = ChildFormItemsGroup.Horizontal
    И ГруппировкаПодчиненныхЭлементовФормы.ГоризонтальнаяВсегда = ChildFormItemsGroup.AlwaysHorizontal
    И ГруппировкаПодчиненныхЭлементовФормы.ГоризонтальнаяЕслиВозможно = ChildFormItemsGroup.HorizontalIfPossible Тогда
    Сообщить("ChildFormItemsGroup.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ChildFormItemsGroup.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ChoiceHistoryOnInput.ROOT" + Символ(9) + Строка(ИсторияВыбораПриВводе));
Сообщить("ChoiceHistoryOnInput.TYPE" + Символ(9) + Строка(ТипЗнч(ИсторияВыбораПриВводе.Авто)));
Сообщить("ChoiceHistoryOnInput.Auto" + Символ(9) + Строка(ИсторияВыбораПриВводе.Авто));
Сообщить("ChoiceHistoryOnInput.DontUse" + Символ(9) + Строка(ИсторияВыбораПриВводе.НеИспользовать));
Если ИсторияВыбораПриВводе.Авто = ChoiceHistoryOnInput.Auto
    И ИсторияВыбораПриВводе.НеИспользовать = ChoiceHistoryOnInput.DontUse Тогда
    Сообщить("ChoiceHistoryOnInput.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ChoiceHistoryOnInput.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ChoiceButtonRepresentation.ROOT" + Символ(9) + Строка(ОтображениеКнопкиВыбора));
Сообщить("ChoiceButtonRepresentation.TYPE" + Символ(9) + Строка(ТипЗнч(ОтображениеКнопкиВыбора.Авто)));
Сообщить("ChoiceButtonRepresentation.Auto" + Символ(9) + Строка(ОтображениеКнопкиВыбора.Авто));
Сообщить("ChoiceButtonRepresentation.ShowInDropList" + Символ(9) + Строка(ОтображениеКнопкиВыбора.ОтображатьВВыпадающемСписке));
Сообщить("ChoiceButtonRepresentation.ShowInDropListAndInInputField" + Символ(9) + Строка(ОтображениеКнопкиВыбора.ОтображатьВВыпадающемСпискеИВПолеВвода));
Сообщить("ChoiceButtonRepresentation.ShowInInputField" + Символ(9) + Строка(ОтображениеКнопкиВыбора.ОтображатьВПолеВвода));
Если ОтображениеКнопкиВыбора.Авто = ChoiceButtonRepresentation.Auto
    И ОтображениеКнопкиВыбора.ОтображатьВВыпадающемСписке = ChoiceButtonRepresentation.ShowInDropList
    И ОтображениеКнопкиВыбора.ОтображатьВВыпадающемСпискеИВПолеВвода = ChoiceButtonRepresentation.ShowInDropListAndInInputField
    И ОтображениеКнопкиВыбора.ОтображатьВПолеВвода = ChoiceButtonRepresentation.ShowInInputField Тогда
    Сообщить("ChoiceButtonRepresentation.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ChoiceButtonRepresentation.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("UsualGroupControlRepresentation.ROOT" + Символ(9) + Строка(ОтображениеУправленияОбычнойГруппы));
Сообщить("UsualGroupControlRepresentation.TYPE" + Символ(9) + Строка(ТипЗнч(ОтображениеУправленияОбычнойГруппы.ГиперссылкаЗаголовка)));
Сообщить("UsualGroupControlRepresentation.TitleHyperlink" + Символ(9) + Строка(ОтображениеУправленияОбычнойГруппы.ГиперссылкаЗаголовка));
Сообщить("UsualGroupControlRepresentation.Picture" + Символ(9) + Строка(ОтображениеУправленияОбычнойГруппы.Картинка));
Если ОтображениеУправленияОбычнойГруппы.ГиперссылкаЗаголовка = UsualGroupControlRepresentation.TitleHyperlink
    И ОтображениеУправленияОбычнойГруппы.Картинка = UsualGroupControlRepresentation.Picture Тогда
    Сообщить("UsualGroupControlRepresentation.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("UsualGroupControlRepresentation.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("UsualGroupBehavior.ROOT" + Символ(9) + Строка(ПоведениеОбычнойГруппы));
Сообщить("UsualGroupBehavior.TYPE" + Символ(9) + Строка(ТипЗнч(ПоведениеОбычнойГруппы.Авто)));
Сообщить("UsualGroupBehavior.Auto" + Символ(9) + Строка(ПоведениеОбычнойГруппы.Авто));
Сообщить("UsualGroupBehavior.PopUp" + Символ(9) + Строка(ПоведениеОбычнойГруппы.Всплывающая));
Сообщить("UsualGroupBehavior.Usual" + Символ(9) + Строка(ПоведениеОбычнойГруппы.Обычное));
Сообщить("UsualGroupBehavior.Collapsible" + Символ(9) + Строка(ПоведениеОбычнойГруппы.Свертываемая));
Если ПоведениеОбычнойГруппы.Авто = UsualGroupBehavior.Auto
    И ПоведениеОбычнойГруппы.Всплывающая = UsualGroupBehavior.PopUp
    И ПоведениеОбычнойГруппы.Обычное = UsualGroupBehavior.Usual
    И ПоведениеОбычнойГруппы.Свертываемая = UsualGroupBehavior.Collapsible Тогда
    Сообщить("UsualGroupBehavior.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("UsualGroupBehavior.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("FormItemTitleLocation.ROOT" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы));
Сообщить("FormItemTitleLocation.TYPE" + Символ(9) + Строка(ТипЗнч(ПоложениеЗаголовкаЭлементаФормы.Авто)));
Сообщить("FormItemTitleLocation.Auto" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Авто));
Сообщить("FormItemTitleLocation.Top" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Верх));
Сообщить("FormItemTitleLocation.Left" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Лево));
Сообщить("FormItemTitleLocation.None" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Нет));
Сообщить("FormItemTitleLocation.Bottom" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Низ));
Сообщить("FormItemTitleLocation.Right" + Символ(9) + Строка(ПоложениеЗаголовкаЭлементаФормы.Право));
Если ПоложениеЗаголовкаЭлементаФормы.Авто = FormItemTitleLocation.Auto
    И ПоложениеЗаголовкаЭлементаФормы.Верх = FormItemTitleLocation.Top
    И ПоложениеЗаголовкаЭлементаФормы.Лево = FormItemTitleLocation.Left
    И ПоложениеЗаголовкаЭлементаФормы.Нет = FormItemTitleLocation.None
    И ПоложениеЗаголовкаЭлементаФормы.Низ = FormItemTitleLocation.Bottom
    И ПоложениеЗаголовкаЭлементаФормы.Право = FormItemTitleLocation.Right Тогда
    Сообщить("FormItemTitleLocation.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("FormItemTitleLocation.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("ButtonLocationInCommandBar.ROOT" + Символ(9) + Строка(ПоложениеКнопкиВКоманднойПанели));
Сообщить("ButtonLocationInCommandBar.TYPE" + Символ(9) + Строка(ТипЗнч(ПоложениеКнопкиВКоманднойПанели.Авто)));
Сообщить("ButtonLocationInCommandBar.Auto" + Символ(9) + Строка(ПоложениеКнопкиВКоманднойПанели.Авто));
Сообщить("ButtonLocationInCommandBar.InAdditionalSubmenu" + Символ(9) + Строка(ПоложениеКнопкиВКоманднойПанели.ВДополнительномПодменю));
Сообщить("ButtonLocationInCommandBar.InCommandBar" + Символ(9) + Строка(ПоложениеКнопкиВКоманднойПанели.ВКоманднойПанели));
Сообщить("ButtonLocationInCommandBar.InCommandBarAndInAdditionalSubmenu" + Символ(9) + Строка(ПоложениеКнопкиВКоманднойПанели.ВКоманднойПанелиИВДополнительномПодменю));
Если ПоложениеКнопкиВКоманднойПанели.Авто = ButtonLocationInCommandBar.Auto
    И ПоложениеКнопкиВКоманднойПанели.ВДополнительномПодменю = ButtonLocationInCommandBar.InAdditionalSubmenu
    И ПоложениеКнопкиВКоманднойПанели.ВКоманднойПанели = ButtonLocationInCommandBar.InCommandBar
    И ПоложениеКнопкиВКоманднойПанели.ВКоманднойПанелиИВДополнительномПодменю = ButtonLocationInCommandBar.InCommandBarAndInAdditionalSubmenu Тогда
    Сообщить("ButtonLocationInCommandBar.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("ButtonLocationInCommandBar.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("DataCompositionResourcesPlacement.ROOT" + Символ(9) + Строка(РасположениеРесурсовКомпоновкиДанных));
Сообщить("DataCompositionResourcesPlacement.TYPE" + Символ(9) + Строка(ТипЗнч(РасположениеРесурсовКомпоновкиДанных.Вертикально)));
Сообщить("DataCompositionResourcesPlacement.Vertically" + Символ(9) + Строка(РасположениеРесурсовКомпоновкиДанных.Вертикально));
Сообщить("DataCompositionResourcesPlacement.Horizontally" + Символ(9) + Строка(РасположениеРесурсовКомпоновкиДанных.Горизонтально));
Если РасположениеРесурсовКомпоновкиДанных.Вертикально = DataCompositionResourcesPlacement.Vertically
    И РасположениеРесурсовКомпоновкиДанных.Горизонтально = DataCompositionResourcesPlacement.Horizontally Тогда
    Сообщить("DataCompositionResourcesPlacement.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("DataCompositionResourcesPlacement.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

Сообщить("DataCompositionPeriodAdditionType.ROOT" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных));
Сообщить("DataCompositionPeriodAdditionType.TYPE" + Символ(9) + Строка(ТипЗнч(ТипДополненияПериодаКомпоновкиДанных.БезДополнения)));
Сообщить("DataCompositionPeriodAdditionType.None" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.БезДополнения));
Сообщить("DataCompositionPeriodAdditionType.Year" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Год));
Сообщить("DataCompositionPeriodAdditionType.YearSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ГодОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.YearSinceBeginOfPeriod445" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ГодОтНачалаПериода445));
Сообщить("DataCompositionPeriodAdditionType.TenDays" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Декада));
Сообщить("DataCompositionPeriodAdditionType.Day" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.День));
Сообщить("DataCompositionPeriodAdditionType.DaySinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ДеньОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.Quarter" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Квартал));
Сообщить("DataCompositionPeriodAdditionType.QuarterSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.КварталОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.QuarterSinceBeginOfPeriod445" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.КварталОтНачалаПериода445));
Сообщить("DataCompositionPeriodAdditionType.Month" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Месяц));
Сообщить("DataCompositionPeriodAdditionType.MonthSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.МесяцОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.MonthSinceBeginOfPeriod445" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.МесяцОтНачалаПериода445));
Сообщить("DataCompositionPeriodAdditionType.Minute" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Минута));
Сообщить("DataCompositionPeriodAdditionType.MinuteSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.МинутаОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.Week" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Неделя));
Сообщить("DataCompositionPeriodAdditionType.WeekSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.НеделяОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.HalfYear" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Полугодие));
Сообщить("DataCompositionPeriodAdditionType.HalfYearSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ПолугодиеОтНачалаПериода));
Сообщить("DataCompositionPeriodAdditionType.HalfYearSinceBeginOfPeriod445" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ПолугодиеОтНачалаПериода445));
Сообщить("DataCompositionPeriodAdditionType.Second" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Секунда));
Сообщить("DataCompositionPeriodAdditionType.Hour" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.Час));
Сообщить("DataCompositionPeriodAdditionType.HourSinceBeginOfPeriod" + Символ(9) + Строка(ТипДополненияПериодаКомпоновкиДанных.ЧасОтНачалаПериода));
Если ТипДополненияПериодаКомпоновкиДанных.БезДополнения = DataCompositionPeriodAdditionType.None
    И ТипДополненияПериодаКомпоновкиДанных.Год = DataCompositionPeriodAdditionType.Year
    И ТипДополненияПериодаКомпоновкиДанных.ГодОтНачалаПериода = DataCompositionPeriodAdditionType.YearSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.ГодОтНачалаПериода445 = DataCompositionPeriodAdditionType.YearSinceBeginOfPeriod445
    И ТипДополненияПериодаКомпоновкиДанных.Декада = DataCompositionPeriodAdditionType.TenDays
    И ТипДополненияПериодаКомпоновкиДанных.День = DataCompositionPeriodAdditionType.Day
    И ТипДополненияПериодаКомпоновкиДанных.ДеньОтНачалаПериода = DataCompositionPeriodAdditionType.DaySinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.Квартал = DataCompositionPeriodAdditionType.Quarter
    И ТипДополненияПериодаКомпоновкиДанных.КварталОтНачалаПериода = DataCompositionPeriodAdditionType.QuarterSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.КварталОтНачалаПериода445 = DataCompositionPeriodAdditionType.QuarterSinceBeginOfPeriod445
    И ТипДополненияПериодаКомпоновкиДанных.Месяц = DataCompositionPeriodAdditionType.Month
    И ТипДополненияПериодаКомпоновкиДанных.МесяцОтНачалаПериода = DataCompositionPeriodAdditionType.MonthSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.МесяцОтНачалаПериода445 = DataCompositionPeriodAdditionType.MonthSinceBeginOfPeriod445
    И ТипДополненияПериодаКомпоновкиДанных.Минута = DataCompositionPeriodAdditionType.Minute
    И ТипДополненияПериодаКомпоновкиДанных.МинутаОтНачалаПериода = DataCompositionPeriodAdditionType.MinuteSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.Неделя = DataCompositionPeriodAdditionType.Week
    И ТипДополненияПериодаКомпоновкиДанных.НеделяОтНачалаПериода = DataCompositionPeriodAdditionType.WeekSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.Полугодие = DataCompositionPeriodAdditionType.HalfYear
    И ТипДополненияПериодаКомпоновкиДанных.ПолугодиеОтНачалаПериода = DataCompositionPeriodAdditionType.HalfYearSinceBeginOfPeriod
    И ТипДополненияПериодаКомпоновкиДанных.ПолугодиеОтНачалаПериода445 = DataCompositionPeriodAdditionType.HalfYearSinceBeginOfPeriod445
    И ТипДополненияПериодаКомпоновкиДанных.Секунда = DataCompositionPeriodAdditionType.Second
    И ТипДополненияПериодаКомпоновкиДанных.Час = DataCompositionPeriodAdditionType.Hour
    И ТипДополненияПериодаКомпоновкиДанных.ЧасОтНачалаПериода = DataCompositionPeriodAdditionType.HourSinceBeginOfPeriod Тогда
    Сообщить("DataCompositionPeriodAdditionType.ALIASES" + Символ(9) + "PASS");
Иначе
    Сообщить("DataCompositionPeriodAdditionType.ALIASES" + Символ(9) + "FAIL");
КонецЕсли;

