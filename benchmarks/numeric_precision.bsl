// Девять численных задач. Результаты — наблюдения, не эталон платформы 1С.
Функция SmallRump(Знач a, Знач b)
    b2 = b * b;
    b4 = b2 * b2;
    b6 = b4 * b2;
    b8 = b4 * b4;
    a2 = a * a;
    T1 = 333.75 * b6;
    Inner = 11.0 * a2 * b2 - b6 - 121.0 * b4 - 2.0;
    T2 = a2 * Inner;
    T3 = 5.5 * b8;
    T4 = a / (2.0 * b);
    S1 = T1 + T2;
    S2 = S1 + T3;
    Result = S2 + T4;
    Возврат Result;
КонецФункции

Функция MultiplyRounding()
    x = 1234567890.123456789;
    y = 9876543210.987654321;
    Возврат x * y;
КонецФункции

Функция HarmonicSeries()
    sum = 0.0;
    n = 1.0;
    Пока n <= 10000000.0 Цикл
        sum = sum + (1.0 / n);
        n = n + 1.0;
    КонецЦикла;
    Возврат sum;
КонецФункции

Функция BinetFibonacci()
    sqrt5 = 2.2360679774997896964091736687;
    phi = (1.0 + sqrt5) / 2.0;
    psi = (1.0 - sqrt5) / 2.0;
    phi_pow = 1.0;
    psi_pow = 1.0;
    i = 1;
    Пока i <= 75 Цикл
        phi_pow = phi_pow * phi;
        psi_pow = psi_pow * psi;
        i = i + 1;
    КонецЦикла;
    Возврат (phi_pow - psi_pow) / sqrt5;
КонецФункции

Функция SubtractiveCancellation()
    root_x = 10000000000000.0;
    root_x_plus_1 = 10000000000000.00000000000005;
    Возврат root_x_plus_1 - root_x;
КонецФункции

Функция CalculateEuler()
    e_val = 1.0;
    term = 1.0;
    i = 1.0;
    Пока term > 0.0 Цикл
        term = term / i;
        Если term = 0.0 Тогда
            Прервать;
        КонецЕсли;
        e_val = e_val + term;
        i = i + 1.0;
    КонецЦикла;
    Возврат e_val;
КонецФункции

Функция Sqrt2Newton()
    S = 2.0;
    x = 1.0;
    prev_x = 0.0;
    Пока x <> prev_x Цикл
        prev_x = x;
        // Ограничение точности обеспечивает остановку по равенству.
        x = Окр(0.5 * (x + (S / x)), 27);
    КонецЦикла;
    Возврат x;
КонецФункции

Функция ArcTanTaylor(Знач x)
    sum = 0.0;
    term = x;
    x2 = x * x;
    n = 1.0;
    sign = 1.0;
    Пока term > 0.0 Цикл
        sum = sum + sign * (term / n);
        // Член должен обнулиться и при точном десятичном умножении.
        term = Окр(term * x2, 27);
        n = n + 2.0;
        sign = 0.0 - sign;
    КонецЦикла;
    Возврат sum;
КонецФункции

Функция CalculatePi()
    part1 = 16.0 * ArcTanTaylor(1.0 / 5.0);
    part2 = 4.0 * ArcTanTaylor(1.0 / 239.0);
    Возврат part1 - part2;
КонецФункции

Функция MortgageAnnuity()
    Principal = 1000000000.0;
    AnnualRate = 0.035;
    Months = 360.0;
    r = AnnualRate / 12.0;
    power = 1.0;
    i = 1.0;
    Пока i <= Months Цикл
        power = power * (1.0 + r);
        i = i + 1.0;
    КонецЦикла;
    PMT = Principal * (r * power) / (power - 1.0);
    Возврат PMT;
КонецФункции

Т1 = ТекущаяУниверсальнаяДатаВМиллисекундах();
Сообщить("small_rump: " + SmallRump(77.0, 33.0));
Сообщить("multiply_rounding: " + MultiplyRounding());
Сообщить("harmonic_series: " + HarmonicSeries());
Сообщить("binet_fibonacci: " + BinetFibonacci());
Сообщить("subtractive_cancellation: " + SubtractiveCancellation());
Сообщить("euler: " + CalculateEuler());
Сообщить("sqrt2_newton: " + Sqrt2Newton());
Сообщить("pi_machin: " + CalculatePi());
Сообщить("mortgage_annuity: " + MortgageAnnuity());
Сообщить(Формат(ТекущаяУниверсальнаяДатаВМиллисекундах() - Т1, "ЧГ=0"));
