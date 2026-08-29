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

CHUNK = re.compile(r'^\.chunk (\d+) params=(\d+) locals=(\d+) regs=\d+ argmodes=\[([^\]]*)\]')
MODULE_VARS = re.compile(r'^\.module-vars (\d+)')
OBJ = re.compile(r'\bobj=(\d+)')
INSTR = re.compile(r'^\s+(\d{4}) (\w+)(.*)$')
ARGMODES = re.compile(r'^\s+(\d+) \[(.*)\]\s*$')
HANDLER = re.compile(r'^\s+\d+ (\d+) (\d+) (\d+)\s*(;.*)?$')
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
    """Чанки листинга со всем, что нужно классификатору."""
    out, cur, instrs, modes, in_argmodes = [], None, [], {}, False
    ranges, in_handlers, module_vars = [], False, 0
    for line in listing.splitlines():
        m = MODULE_VARS.match(line)
        if m:
            module_vars = int(m.group(1))
            continue
        m = CHUNK.match(line)
        if m:
            if cur is not None:
                out.append((*cur, instrs, modes, ranges))
            by_val = [x.strip() == 'value' for x in m.group(4).split(',') if x.strip()]
            cur = (int(m.group(1)), int(m.group(2)), int(m.group(3)), by_val, module_vars)
            instrs, modes, ranges = [], {}, []
            in_argmodes = in_handlers = False
            continue
        if line.startswith('  .handlers'):
            in_handlers, in_argmodes = True, False
            continue
        if in_handlers:
            m = HANDLER.match(line)
            if m:
                ranges.append((int(m.group(1)), int(m.group(2)), int(m.group(3))))
                continue
            in_handlers = False
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
    if cur is not None:
        out.append((*cur, instrs, modes, ranges))
    return out


# Почему копия осталась. Деления на «из локали» и «из временного» мало:
# оно отвечает на вопрос «есть ли что перенаправлять», но не на вопрос
# «чем эту копию снимать». Причин три, и работы за ними разные.
REASON_SEMANTIC = 'обязательна: окно Call (Знач)'
REASON_WIDE = 'широкое окно: нужна раскладка регистров'
NARROW = 'узкое окно нативного, '


def narrow_reason(i, dst, recv, ctx):
    """Почему перестановка базы НЕ сработала на узком окне.

    Проверяется всё, что видно в листинге: соседство, границы и тело
    защищённого диапазона, точность приёмника и совпадение его с
    получателем. Не видна отсюда только живучесть приёмника после
    вызова — она и остаётся единственным необъяснённым остатком, и
    называть его «должен был сняться» нельзя: у копии есть ещё одно
    условие, которого этот разбор не проверяет.
    """
    if not ctx['adjacent']:
        return NARROW + 'копия не соседняя вызову'
    if ctx['boundary'](i):
        return NARROW + 'граница защищённого диапазона'
    if ctx['protected'](i):
        return NARROW + 'внутри Попытка'
    if not ctx['exact'](dst):
        return NARROW + 'приёмник неточен'
    if recv == dst:
        return NARROW + 'приёмник он же получатель'
    return NARROW + 'остаётся живучесть (не видна в листинге)'


def classify(path):
    """Копии в окно аргументов по источнику и по причине."""
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
    by_reason = {}
    for idx, n_params, n_locals, by_val, module_vars, instrs, modes, ranges in chunks_of(p.stdout):
        ctx = {
            'protected': lambda pc: any(lo <= pc < hi for lo, hi, _ in ranges),
            'boundary': lambda pc: any(pc in (lo, hi, h) for lo, hi, h in ranges),
            # Точность — то же правило, что у `analysis::exact_reg`:
            # псевдоним параметра по ссылке или перекрытие модульным
            # слотом (перекрывается только кадр нулевого уровня).
            'exact': lambda r: not (r < n_params and not by_val[r])
            and not (idx == 0 and r < module_vars),
        }
        for i, (op, text) in enumerate(instrs):
            if op != 'Move':
                continue
            m = MOVE.search(text)
            if not m:
                continue
            dst, src = int(m.group(1)), int(m.group(2))
            for j in range(i + 1, len(instrs)):
                op2, text2 = instrs[j]
                if op2.startswith('Call') or op2 == 'CreateObject':
                    w = window_of(text2, modes)
                    if w is not None and dst in w:
                        if src < n_locals:
                            loc += 1
                        else:
                            tmp += 1
                        if op2 in ('Call', 'CallImported'):
                            key = REASON_SEMANTIC
                        elif len(w) > 1:
                            key = REASON_WIDE
                        else:
                            recv = OBJ.search(text2)
                            ctx['adjacent'] = j == i + 1
                            key = narrow_reason(
                                i, dst, int(recv.group(1)) if recv else None, ctx
                            )
                        by_reason[key] = by_reason.get(key, 0) + 1
                        break
                w2 = WRITES.search(text2)
                if w2 and int(w2.group(1)) == dst:
                    break
    return loc, tmp, by_reason


def main():
    tot_loc = tot_tmp = ok = failed = 0
    reasons = {}
    for path in sorted(glob.glob('benchmarks/*.bsl') + glob.glob('tests/conformance/fixtures/*.bsl')):
        got = classify(path)
        if got is None:
            failed += 1
            continue
        ok += 1
        loc, tmp, by_reason = got
        tot_loc += loc
        tot_tmp += tmp
        for k, v in by_reason.items():
            reasons[k] = reasons.get(k, 0) + v

    print(f'скомпилировано {ok}, не скомпилировано {failed}')
    print(f'\nпо источнику:  из локали {tot_loc}, из временного регистра {tot_tmp}')
    print('\nпо причине (чем эту копию снимать):')
    for key in sorted(reasons):
        print(f'  {key:32} {reasons[key]:6}')
    print(
        '\nАбсолютные числа зависят от методики разрешения достигающих\n'
        'определений и в выводы не берутся: несут их нули и порядки.'
    )
    if failed:
        # Несобравшийся скрипт — это отказ, а не «копий не нашлось».
        # Замер, сообщающий о пропуске и завершающийся нулём, читается как
        # успешный.
        sys.exit(1)


main()
