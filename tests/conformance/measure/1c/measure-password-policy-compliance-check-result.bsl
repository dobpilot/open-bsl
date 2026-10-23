// PLATFORM-ONLY: проба фиксирует полное перечисление результатов проверки пароля.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "password-policy-compliance-check-result-1");
М("PASSWORD.POLICY.RESULT.MIN_LENGTH",
	РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины);
М("PASSWORD.POLICY.RESULT.REUSE",
	РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних);
М("PASSWORD.POLICY.RESULT.COMPROMISE",
	РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямПроверкиРаскрытия);
М("PASSWORD.POLICY.RESULT.COMPLEXITY",
	РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямСложности);
М("PASSWORD.POLICY.RESULT.TYPE",
	ТипЗнч(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины));

АнглийскиеИмена = "ERROR";
Попытка
	Если PasswordPolicyComplianceCheckResult.DoesNotSatisfyMinLengthRequirements
		= РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины
		И PasswordPolicyComplianceCheckResult.DoesNotSatisfyReuseLimitRequirements
		= РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних
		И PasswordPolicyComplianceCheckResult.DoesNotSatisfyCompromiseCheckRequirements
		= РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямПроверкиРаскрытия
		И PasswordPolicyComplianceCheckResult.DoesNotSatisfyComplexityRequirements
		= РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямСложности Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("PASSWORD.POLICY.RESULT.ENGLISH", АнглийскиеИмена);
М("file.end", "password-policy-compliance-check-result-1");
