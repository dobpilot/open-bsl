// PLATFORM-ONLY: проба фиксирует закрытое перечисление ОтображениеФигурыКнопки.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "button-shape-representation-1");
М("BUTTON.SHAPE.AUTO", ОтображениеФигурыКнопки.Авто);
М("BUTTON.SHAPE.ALWAYS", ОтображениеФигурыКнопки.Всегда);
М("BUTTON.SHAPE.NONE", ОтображениеФигурыКнопки.Нет);
М("BUTTON.SHAPE.WHEN_ACTIVE", ОтображениеФигурыКнопки.ПриАктивности);
М("BUTTON.SHAPE.TYPE", ТипЗнч(ОтображениеФигурыКнопки.Авто));
М("BUTTON.SHAPE.ALIAS.AUTO", ОтображениеФигурыКнопки.Авто = ButtonShapeRepresentation.Auto);
М("BUTTON.SHAPE.ALIAS.ALWAYS", ОтображениеФигурыКнопки.Всегда = ButtonShapeRepresentation.Always);
М("BUTTON.SHAPE.ALIAS.NONE", ОтображениеФигурыКнопки.Нет = ButtonShapeRepresentation.None);
М("BUTTON.SHAPE.ALIAS.WHEN_ACTIVE", ОтображениеФигурыКнопки.ПриАктивности = ButtonShapeRepresentation.WhenActive);
М("file.end", "button-shape-representation-1");
