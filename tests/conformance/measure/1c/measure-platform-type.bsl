// PLATFORM-ONLY: проба фиксирует полное системное перечисление типов платформы.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "platform-type-1");
М("PLATFORM.TYPE.ANDROID.ARM", ТипПлатформы.Android_ARM);
М("PLATFORM.TYPE.ANDROID.ARM64", ТипПлатформы.Android_ARM_64);
М("PLATFORM.TYPE.ANDROID.X86", ТипПлатформы.Android_x86);
М("PLATFORM.TYPE.ANDROID.X86_64", ТипПлатформы.Android_x86_64);
М("PLATFORM.TYPE.IOS.ARM", ТипПлатформы.iOS_ARM);
М("PLATFORM.TYPE.IOS.ARM64", ТипПлатформы.iOS_ARM_64);
М("PLATFORM.TYPE.LINUX.ARM64", ТипПлатформы.Linux_ARM64);
М("PLATFORM.TYPE.LINUX.E2K", ТипПлатформы.Linux_E2K);
М("PLATFORM.TYPE.LINUX.X86", ТипПлатформы.Linux_x86);
М("PLATFORM.TYPE.LINUX.X86_64", ТипПлатформы.Linux_x86_64);
М("PLATFORM.TYPE.MACOS.X86", ТипПлатформы.MacOS_x86);
М("PLATFORM.TYPE.MACOS.X86_64", ТипПлатформы.MacOS_x86_64);
М("PLATFORM.TYPE.WINDOWS.X86", ТипПлатформы.Windows_x86);
М("PLATFORM.TYPE.WINDOWS.X86_64", ТипПлатформы.Windows_x86_64);
М("PLATFORM.TYPE.WINRT.ARM", ТипПлатформы.WinRT_ARM);
М("PLATFORM.TYPE.WINRT.X86", ТипПлатформы.WinRT_x86);
М("PLATFORM.TYPE.WINRT.X86_64", ТипПлатформы.WinRT_x86_64);
М("PLATFORM.TYPE.TYPE", ТипЗнч(ТипПлатформы.Linux_x86_64));
М("PLATFORM.TYPE.ENGLISH", PlatformType.Linux_x86_64 = ТипПлатформы.Linux_x86_64);
М("file.end", "platform-type-1");
