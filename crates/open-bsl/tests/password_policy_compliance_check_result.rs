use open_bsl::Engine;

#[test]
fn password_policy_compliance_check_result_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины)
        <> "Does not meet the minimum length requirements"
    Или Строка(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних)
        <> "Does not meet the requirements for reuse limit among recent ones"
    Или Строка(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямПроверкиРаскрытия)
        <> "Does not meet compromise check requirements"
    Или Строка(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямСложности)
        <> "Does not meet the complexity requirements"
    Или Строка(ТипЗнч(РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины))
        <> "PasswordPolicyComplianceCheckResult"
    Или PasswordPolicyComplianceCheckResult.DoesNotSatisfyMinLengthRequirements
        <> РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямМинимальнойДлины
    Или PasswordPolicyComplianceCheckResult.DoesNotSatisfyReuseLimitRequirements
        <> РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних
    Или PasswordPolicyComplianceCheckResult.DoesNotSatisfyCompromiseCheckRequirements
        <> РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямПроверкиРаскрытия
    Или PasswordPolicyComplianceCheckResult.DoesNotSatisfyComplexityRequirements
        <> РезультатПроверкиСоответствияПароляПолитике.НеСоответствуетТребованиямСложности Тогда
    ВызватьИсключение "password-policy-compliance-check-result";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
