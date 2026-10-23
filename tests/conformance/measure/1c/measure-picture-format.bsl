// PLATFORM-ONLY: проба фиксирует закрытое перечисление ФорматКартинки.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "picture-format-1");
М("PICTURE.FORMAT.BMP", ФорматКартинки.BMP);
М("PICTURE.FORMAT.EMF", ФорматКартинки.EMF);
М("PICTURE.FORMAT.GIF", ФорматКартинки.GIF);
М("PICTURE.FORMAT.ICON", ФорматКартинки.Icon);
М("PICTURE.FORMAT.JPEG", ФорматКартинки.JPEG);
М("PICTURE.FORMAT.PNG", ФорматКартинки.PNG);
М("PICTURE.FORMAT.SVG", ФорматКартинки.SVG);
М("PICTURE.FORMAT.TIFF", ФорматКартинки.TIFF);
М("PICTURE.FORMAT.WMF", ФорматКартинки.WMF);
М("PICTURE.FORMAT.UNKNOWN", ФорматКартинки.НеизвестныйФормат);
М("PICTURE.FORMAT.TYPE", ТипЗнч(ФорматКартинки.PNG));
М("PICTURE.FORMAT.ALIAS.BMP", ФорматКартинки.BMP = PictureFormat.BMP);
М("PICTURE.FORMAT.ALIAS.EMF", ФорматКартинки.EMF = PictureFormat.EMF);
М("PICTURE.FORMAT.ALIAS.GIF", ФорматКартинки.GIF = PictureFormat.GIF);
М("PICTURE.FORMAT.ALIAS.ICON", ФорматКартинки.Icon = PictureFormat.Icon);
М("PICTURE.FORMAT.ALIAS.JPEG", ФорматКартинки.JPEG = PictureFormat.JPEG);
М("PICTURE.FORMAT.ALIAS.PNG", ФорматКартинки.PNG = PictureFormat.PNG);
М("PICTURE.FORMAT.ALIAS.SVG", ФорматКартинки.SVG = PictureFormat.SVG);
М("PICTURE.FORMAT.ALIAS.TIFF", ФорматКартинки.TIFF = PictureFormat.TIFF);
М("PICTURE.FORMAT.ALIAS.WMF", ФорматКартинки.WMF = PictureFormat.WMF);
М("PICTURE.FORMAT.ALIAS.UNKNOWN", ФорматКартинки.НеизвестныйФормат = PictureFormat.UnknownFormat);
М("file.end", "picture-format-1");
