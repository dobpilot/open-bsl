"""Что питает копии в окно аргументов вызова.

Вторая половина шага 6 плана (docs/ssa-hotspot-analysis.md) предлагает
снимать такие копии «выбором операндов вызова» — заставив источник копии
писать прямо в слот окна. Средство это имеет предмет только там, где
источник есть ИНСТРУКЦИЯ, которую можно перенаправить, то есть временный
регистр. Если же копия читает локальную переменную, перенаправлять нечего:
значение уже лежит в своём слоте, а окно требует его по смещению.

Скрипт разделяет копии по этому признаку на всём корпусе. Считается по
листингу УЖЕ оптимизированного кодогена (`--optimize=copy-elim`): вопрос
именно в том, что проход оставил после себя.

    python3 benchmarks/window-copies.py
"""

import re, subprocess, sys, glob

CLI = 'target/release/bsl-cli'
chunk_re = re.compile(r'^\.chunk \d+ params=(\d+) locals=(\d+)')
ins_re = re.compile(r'^\s+(\d{4}) (\w+)(.*)$')
call_re = re.compile(r'base=(\d+) count=(\d+)')

tot_local = tot_temp = 0
rows = []
for f in sorted(glob.glob('benchmarks/*.bsl') + glob.glob('tests/conformance/fixtures/*.bsl')):
    p = subprocess.run([CLI, '--optimize=copy-elim', '--emit-bytecode', f],
                       capture_output=True, text=True)
    if p.returncode != 0:
        continue
    n_locals = 0
    instrs = []          # (op, text)
    chunks = []          # (n_locals, instrs)
    for line in p.stdout.splitlines():
        m = chunk_re.match(line)
        if m:
            if instrs:
                chunks.append((n_locals, instrs)); instrs = []
            n_locals = int(m.group(2))
            continue
        m = ins_re.match(line)
        if m:
            instrs.append((m.group(2), m.group(3)))
    if instrs:
        chunks.append((n_locals, instrs))

    loc = tmp = 0
    for n_locals, ins in chunks:
        for i, (op, txt) in enumerate(ins):
            if op != 'Move':
                continue
            mm = re.search(r'dst=(\d+) src=(\d+)', txt)
            if not mm:
                continue
            dst, src = int(mm.group(1)), int(mm.group(2))
            # Ищем ближайший следующий вызов и смотрим, лежит ли dst в его окне.
            for op2, txt2 in ins[i + 1:i + 4]:
                if not op2.startswith('Call'):
                    continue
                c = call_re.search(txt2)
                if c and int(c.group(1)) <= dst < int(c.group(1)) + int(c.group(2)):
                    if src < n_locals:
                        loc += 1
                    else:
                        tmp += 1
                break
    if loc or tmp:
        rows.append((f.split('/')[-1][:-4], loc, tmp))
    tot_local += loc; tot_temp += tmp

rows.sort(key=lambda r: -(r[1] + r[2]))
print(f"{'скрипт':28} {'из локали':>10} {'из временного':>14}")
for n, a, b in rows[:12]:
    print(f"{n:28} {a:10} {b:14}")
print(f"\nВСЕГО: из локали {tot_local}, из временного регистра {tot_temp}")
