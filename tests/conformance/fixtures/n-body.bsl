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
	Message(Energie(bodies));

	For i=1 To 50000000 Do
		Advance(bodies,0.01);
	EndDo;	

	Message(Energie(bodies));

EndFunction

Main();