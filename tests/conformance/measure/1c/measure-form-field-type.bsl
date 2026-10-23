// Полный состав системного перечисления ВидПоляФормы.

Процедура М(Ид, Русское, Английское)
	Сообщить(Ид + Символ(9) + Строка(ТипЗнч(Русское)) + Символ(9)
		+ Строка(Русское) + Символ(9) + ?(Русское = Английское, "Yes", "No"));
КонецПроцедуры

М("FORM_FIELD.HTML", ВидПоляФормы.ПолеHTMLДокумента, FormFieldType.HTMLDocumentField);
М("FORM_FIELD.PDF", ВидПоляФормы.ПолеPDFДокумента, FormFieldType.PDFDocumentField);
М("FORM_FIELD.INPUT", ВидПоляФормы.ПолеВвода, FormFieldType.InputField);
М("FORM_FIELD.GEOGRAPHICAL", ВидПоляФормы.ПолеГеографическойСхемы, FormFieldType.GeographicalSchemaField);
М("FORM_FIELD.GRAPHICAL", ВидПоляФормы.ПолеГрафическойСхемы, FormFieldType.GraphicalSchemaField);
М("FORM_FIELD.DENDROGRAM", ВидПоляФормы.ПолеДендрограммы, FormFieldType.DendrogramField);
М("FORM_FIELD.CHART", ВидПоляФормы.ПолеДиаграммы, FormFieldType.ChartField);
М("FORM_FIELD.GANTT", ВидПоляФормы.ПолеДиаграммыГанта, FormFieldType.GanttChartField);
М("FORM_FIELD.PROGRESS", ВидПоляФормы.ПолеИндикатора, FormFieldType.ProgressBarField);
М("FORM_FIELD.CALENDAR", ВидПоляФормы.ПолеКалендаря, FormFieldType.CalendarField);
М("FORM_FIELD.PICTURE", ВидПоляФормы.ПолеКартинки, FormFieldType.PictureField);
М("FORM_FIELD.LABEL", ВидПоляФормы.ПолеНадписи, FormFieldType.LabelField);
М("FORM_FIELD.RADIO", ВидПоляФормы.ПолеПереключателя, FormFieldType.RadioButtonField);
М("FORM_FIELD.PERIOD", ВидПоляФормы.ПолеПериода, FormFieldType.PeriodField);
М("FORM_FIELD.PLANNER", ВидПоляФормы.ПолеПланировщика, FormFieldType.PlannerField);
М("FORM_FIELD.TRACK", ВидПоляФормы.ПолеПолосыРегулирования, FormFieldType.TrackBarField);
М("FORM_FIELD.SPREADSHEET", ВидПоляФормы.ПолеТабличногоДокумента, FormFieldType.SpreadsheetDocumentField);
М("FORM_FIELD.TEXT", ВидПоляФормы.ПолеТекстовогоДокумента, FormFieldType.TextDocumentField);
М("FORM_FIELD.CHECK", ВидПоляФормы.ПолеФлажка, FormFieldType.CheckBoxField);
М("FORM_FIELD.FORMATTED", ВидПоляФормы.ПолеФорматированногоДокумента, FormFieldType.FormattedDocumentField);
