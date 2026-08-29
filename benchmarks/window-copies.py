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
    """Чанки листинга: (число локалей, инструкции, таблица режимов)."""
    out, n_locals, instrs, modes, in_argmodes = [], 0, [], {}, False
    ranges, in_handlers = [], False
    for line in listing.splitlines():
        m = CHUNK.match(line)
        if m:
            if instrs:
                out.append((n_locals, instrs, modes, ranges))
            n_locals, instrs, modes = int(m.group(2)), [], {}
            ranges, in_argmodes, in_handlers = [], False, False
            continue
        if line.startswith('  .handlers'):
            in_handlers, in_argmodes = True, False
            continue
        if in_handlers:
            m = HANDLER.match(line)
            if m:
                ranges.append((int(m.group(1)), int(m.group(2))))
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
    if instrs:
        out.append((n_locals, instrs, modes, ranges))
    return out


# Почему копия осталась. Деления на «из локали» и «из временного» мало:
# оно отвечает на вопрос «есть ли что перенаправлять», но не на вопрос
# «чем эту копию снимать». Причин три, и работы за ними разные.
REASON_SEMANTIC = 'обязательна (Знач)'
REASON_WIDE = 'широкое окно'
REASON_NARROW = 'узкое окно нативного'


def reason_for(op, width):
    """Почему копия в окно этого вызова осталась."""
    if op in ('Call', 'CallImported'):
        # Слот окна СТАНОВИТСЯ параметром вызванной функции: копия и есть
        # приватность `Знач`. Снять её нельзя вообще, ни распределением
        # регистров, ни чем-либо ещё, — она не оптимизационный остаток.
        return REASON_SEMANTIC
    if width > 1:
        # Окно обязано быть непрерывным, а источники соседних аргументов
        # лежат где придётся: нужно согласованное размещение группы, то
        # есть распределение регистров.
        return REASON_WIDE
    # Однорегистровое окно нативного вызова перестановка базы обязана была
    # снять. Уцелевшее здесь — не «остаток на потом», а повод посмотреть,
    # что ей помешало.
    return REASON_NARROW


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
    for n_locals, instrs, modes, ranges in chunks_of(p.stdout):
        protected = lambda pc: any(lo <= pc < hi for lo, hi in ranges)
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
                        key = reason_for(op2, len(w))
                        # У узкого нативного окна отдельно считается,
                        # соседняя ли копия: перестановка базы требует
                        # СОСЕДСТВА, иначе между копией и вызовом успевает
                        # вклиниться инструкция, и доказывать пришлось бы
                        # больше.
                        if key == REASON_NARROW:
                            if text2 is not instrs[i + 1][1]:
                                key += ' (не соседняя)'
                            elif protected(i):
                                # `Попытка` — заявленное ограничение
                                # прохода: исключение может сработать на
                                # любой инструкции диапазона, а точной
                                # живучести по каждой его точке нет.
                                key += ' (в Попытка)'
                            else:
                                key += ' (соседняя, вне Попытка)'
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


main()
