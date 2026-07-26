// Ответвление tests/conformance/fixtures/n-body.bsl (задача 2, ревью):
// оригинальный файл был неисполним (50 000 000 итераций Advance) что у
// нас, что в самой 1С — точная десятичная арифметика растит масштаб на
// каждой итерации без границы (см. bsl-number).
//
// ОТКЛОНЕНИЕ ОТ БРИФА: попросили 1000 итераций. Измерено на этом движке
// (`cargo run --release -p bsl-cli -- .../n-body-precision.bsl`, до правки
// на 100): 1000 итераций не просто медленные — они физически не
// завершаются, а падают через ~98 секунд с `RtError` "превышен предел
// масштаба" (MAX_SCALE = 100_000 в bsl-number, см. number.rs). Рост
// масштаба быстрее, чем прикидка "+50 знаков/итерацию" из брифа: даже
// 600 итераций (без ошибки) заняли 79 секунд. Взято 100 итераций — заметно
// больше дымового варианта (3), укладывается в ~1 секунду и гарантированно
// не упирается в MAX_SCALE. Если появится доступ к платформе для снятия
// оракула, стоит СНАЧАЛА подтвердить на этом же движке, что выбранное
// число итераций вообще завершается, прежде чем гнаться за большим
// числом ради "отпечатка всей арифметики".
//
// НЕТ .expected: снятие эталонной энергии требует живой 1С/OneScript,
// недоступной в этой сессии. Файл готов — .expected нужно добавить
// рядом (n-body-precision.expected, одна строка на каждый Message)
// после снятия замера на платформе; до этого раннер (crates/bsl-cli/
// tests/conformance.rs) молча пропускает фикстуры без пары .expected.

Function getConst()
	PI = 3.141592653589793;
	SOLAR_MASS = 4 * PI * PI;
	DAYS_PER_YEAR = 365.24;

	Return New Structure("PI,SOLAR_MASS,DAYS_PER_YEAR", PI, SOLAR_MASS, DAYS_PER_YEAR );
EndFunction

Function getBodies(const)
	
	PI = const.PI;
	SOLAR_MASS = const.SOLAR_MASS;
	DAYS_PER_YEAR = const.DAYS_PER_YEAR;

	bodies = New Array(5);

	// Solar
	bodies[0] = New Structure("x,y,z,vx,vy,vz,mass",0,0,0,0,0,0,SOLAR_MASS);
	// Jupiter
	bodies[1] = New Structure("x,y,z,vx,vy,vz,mass", 
								  4.84143144246472090,
								  -1.16032004402742839,
								  -103622044471123109/1000000000000000000,
								  83003832137201847/50000000000000000000 * DAYS_PER_YEAR,
								  30796044736789617/4000000000000000000 * DAYS_PER_YEAR,
								  -690460016972063023/10000000000000000000000 * DAYS_PER_YEAR,
								  954791938424326609/1000000000000000000000 * SOLAR_MASS
								  );
	// Saturn
	bodies[2] = New Structure("x,y,z,vx,vy,vz,mass",
								8.34336671824457987,
								4.12479856412430479,
								-403523417114321381/1000000000000000000,
								-276742510726862411/100000000000000000000 * DAYS_PER_YEAR,
								249926400617458619/50000000000000000000 * DAYS_PER_YEAR,
								230417297573763929/10000000000000000000000 * DAYS_PER_YEAR,
								71471495166532703/250000000000000000000 * SOLAR_MASS
								 );
	// Uranus
	bodies[3] = New Structure("x,y,z,vx,vy,vz,mass",
								12894369562139131/1000000000000000,
								-18888939252123289/1250000000000000,
								-111653789446327867/500000000000000000,
								148230068782380809/50000000000000000000 * DAYS_PER_YEAR,
								4756943479189619/2000000000000000000 * DAYS_PER_YEAR,
								-74147392135059389/2500000000000000000000 * DAYS_PER_YEAR,
								218312202167578149/5000000000000000000000 * SOLAR_MASS
								 );
	// Neptune
	bodies[4] = New Structure("x,y,z,vx,vy,vz,mass",
								30759394229701833/2000000000000000,
								-259193146099879641/10000000000000000,
								179258772950371181/1000000000000000000,
								134033886245194661/50000000000000000000 * DAYS_PER_YEAR,
								32564834007648459/20000000000000000000 * DAYS_PER_YEAR,
								-95159225451971587/1000000000000000000000 * DAYS_PER_YEAR,
								515138902046611451/10000000000000000000000 * SOLAR_MASS
								 );
							 
	Return bodies; 

EndFunction

Function OffsetMomentum(bodies,const)
	px = 0.0;
	py = 0.0;
	pz = 0.0;
	For Each body In bodies Do
		px = px + body.vx * body.mass;
		py = py + body.vy * body.mass;
		pz = pz + body.vz * body.mass;
	EndDo;
	bodies[0].vx = - px / const.SOLAR_MASS;
	bodies[0].vy = - py / const.SOLAR_MASS;
	bodies[0].vz = - pz / const.SOLAR_MASS;
EndFunction

Function Energie(bodies)
	e = 0.0;

	For i=0 To bodies.Count()-1 Do
		b = bodies[i];
		e = e + 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
		For j=i+1 To bodies.Count()-1 Do
			
			b2 = bodies[j];
			dx = b.x - b2.x;
			dy = b.y - b2.y;
			dz = b.z - b2.z;
			distance = sqrt(dx * dx + dy * dy + dz * dz);
			e = e - (b.mass * b2.mass) / distance
		EndDo;
	EndDo;

	Return e;
EndFunction

Function Advance(bodies,dt)
	For i=0 To bodies.Count()-1 Do
		b = bodies[i];
		For j=i+1 To bodies.Count()-1 Do
			b2 = bodies[j];
			dx = b.x - b2.x;
			dy = b.y - b2.y;
			dz = b.z - b2.z;
			distanced = dx * dx + dy * dy + dz * dz;
			distance  = sqrt(distanced);
			mag = dt / (distanced * distance);

			b.vx = b.vx - dx * b2.mass * mag;
			b.vy = b.vy - dy * b2.mass * mag;
			b.vz = b.vz - dz * b2.mass * mag;
			b2.vx = b2.vx + dx * b.mass * mag;
			b2.vy = b2.vy + dy * b.mass * mag;
			b2.vz = b2.vz + dz * b.mass * mag;
		EndDo;	
	EndDo;
	For i=0 To bodies.Count()-1 Do
		b = bodies[i];
		b.x = b.x + dt*b.vx;
		b.y = b.y + dt*b.vy;
		b.z = b.z + dt*b.vz;			
	EndDo;
EndFunction

Function Main()
	const = getConst();
	bodies = getBodies(const);
	OffsetMomentum(bodies,const);
	Message(Формат(Energie(bodies), "ЧГ=0; ЧРД=."));

	For i=1 To 100 Do
		Advance(bodies,0.01);
	EndDo;	

	Message(Формат(Energie(bodies), "ЧГ=0; ЧРД=."));

EndFunction

Main();