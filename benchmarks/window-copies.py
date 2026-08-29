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

import glob
import re
import subprocess
import sys

CLI = 'target/release/bsl-cli'

CHUNK = re.compile(r'^\.chunk \d+ params=(\d+) locals=(\d+)')
INSTR = re.compile(r'^\s+(\d{4}) (\w+)(.*)$')
ARGMODES = re.compile(r'^\s+(\d+) \[(.*)\]\s*$')
MOVE = re.compile(r'dst=(\d+) src=(\d+)')
# Почти всякая пишущая инструкция называет приёмник полем `dst=`.
WRITES = re.compile(r'\bdst=(\d+)')
WINDOW = re.compile(r'base=(\d+) count=(\d+)')
# У `Call`/`CallImported` ширина окна задана НЕ полем count, а длиной
# набора режимов аргументов: `arg_modes=N` отсылает к таблице `.argmodes`
# чанка. Прежняя редакция скрипта этого не знала и такие вызовы теряла.
ARGMODES_REF = re.compile(r'base=(\d+) arg_modes=(\d+)')


def window_of(text, modes):
    """Позиции окна, которые вызов ЧИТАЕТ, как множество регистров."""
    m = WINDOW.search(text)
    if m:
        base, count = int(m.group(1)), int(m.group(2))
        return set(range(base, base + count))
    m = ARGMODES_REF.search(text)
    if m:
        table = modes.get(int(m.group(2)))
        if table is None:
            return None
        base = int(m.group(1))
        return {base + i for i, is_value in enumerate(table) if is_value}
    return None


def chunks_of(listing):
    """Чанки листинга: (число локалей, инструкции, таблица режимов)."""
    out, n_locals, instrs, modes, in_argmodes = [], 0, [], {}, False
    for line in listing.splitlines():
        m = CHUNK.match(line)
        if m:
            if instrs:
                out.append((n_locals, instrs, modes))
            n_locals, instrs, modes, in_argmodes = int(m.group(2)), [], {}, False
            continue
        if line.startswith('  .argmodes'):
            in_argmodes = True
            continue
        if in_argmodes:
            m = ARGMODES.match(line)
            if m:
                inner = m.group(2).split()
                # Режимы разделены ПРОБЕЛАМИ (`[byref:1 byref:0]`), и из
                # окна читаются ТОЛЬКО позиции `value`: у `byref` и
                # `default` слот занят лишь ради непрерывности диапазона,
                # а значение берётся из алиаса или из пролога умолчаний.
                modes[int(m.group(1))] = [x == 'value' for x in inner]
                continue
            in_argmodes = False
        m = INSTR.match(line)
        if m:
            instrs.append((m.group(2), m.group(3)))
    if instrs:
        out.append((n_locals, instrs, modes))
    return out


def classify(path):
    """Копии в окно аргументов: (из локали, из временного регистра)."""
    p = subprocess.run(
        [CLI, '--optimize=copy-elim', '--emit-bytecode', path],
        capture_output=True, text=True, check=False,
    )
    if p.returncode != 0:
        # Молча пропускать нельзя: пропущенный скрипт читается как «копий
        # нет», а это не то же самое, что «скрипт не скомпилировался».
        print(f'ОШИБКА компиляции: {path}\n{p.stdout}{p.stderr}', file=sys.stderr)
        return None
    loc = tmp = 0
    for n_locals, instrs, modes in chunks_of(p.stdout):
        for i, (op, text) in enumerate(instrs):
            if op != 'Move':
                continue
            m = MOVE.search(text)
            if not m:
                continue
            dst, src = int(m.group(1)), int(m.group(2))
            # Окно ищется до ближайшего вызова, а не в трёх следующих
            # инструкциях: у широкого окна между копией и вызовом стоят
            # копии остальных аргументов, и их бывает больше трёх.
            #
            # По дороге проверяется достигающее определение: если слот
            # переписан ДО вызова, до окна доедет не эта копия, а та, что
            # её затёрла, и считать надо не её.
            # Ищем ПЕРВОГО потребителя слота, а не первый попавшийся
            # вызов: между копией и её вызовом может стоять чужой вызов с
            # другим окном. Останавливаемся, когда слот прочитан окном
            # (считаем) или перезаписан кем угодно (не считаем — до окна
            # доедет не эта копия).
            for op2, text2 in instrs[i + 1:]:
                if op2.startswith('Call') or op2 == 'CreateObject':
                    w = window_of(text2, modes)
                    if w is not None and dst in w:
                        if src < n_locals:
                            loc += 1
                        else:
                            tmp += 1
                        break
                w2 = WRITES.search(text2)
                if w2 and int(w2.group(1)) == dst:
                    break
    return loc, tmp


def main():
    rows, tot_loc, tot_tmp, ok, failed = [], 0, 0, 0, 0
    for path in sorted(glob.glob('benchmarks/*.bsl') + glob.glob('tests/conformance/fixtures/*.bsl')):
        got = classify(path)
        if got is None:
            failed += 1
            continue
        ok += 1
        loc, tmp = got
        tot_loc += loc
        tot_tmp += tmp
        if loc or tmp:
            rows.append((path.rsplit('/', 1)[-1][:-4], loc, tmp))
    rows.sort(key=lambda r: -(r[1] + r[2]))
    print(f"{'скрипт':28} {'из локали':>10} {'из временного':>14}")
    for name, loc, tmp in rows[:12]:
        print(f'{name:28} {loc:10} {tmp:14}')
    print(f'\nскомпилировано {ok}, не скомпилировано {failed}')
    print(f'ВСЕГО: из локали {tot_loc}, из временного регистра {tot_tmp}')


main()
