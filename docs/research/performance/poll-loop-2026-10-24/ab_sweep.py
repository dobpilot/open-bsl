#!/usr/bin/env python3
"""A/B двух бинарников по набору копий каждого из них.

Использование: ab_sweep.py BASE CANDIDATE КОПИЙ СЦЕНАРИЙ... > сырые.csv

Время горячих сценариев зависит от того, какой копией файла запущен
бинарник: побайтно одинаковые копии по путям одной длины стабильно дают
на goto_bench 500, 580 и 700 мс (физическое размещение страниц кода).
Одна копия на сторону — случайная выборка из этого разброса. Поэтому
каждая сторона копируется КОПИЙ раз в `/tmp/abbin/`, запуски идут по всем
копиям с чередованием сторон, а сравниваются средние по копиям. Вывод без
времени сверяется между сторонами.
"""
import csv, os, re, statistics, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SCRIPTS = ROOT / "target/full-ab-2026-10-23/scripts"
import shutil
base, candidate, count, *scenarios = sys.argv[1:]
# `AB_STORE` — для крупных сборок с отладочной информацией, которые не
# помещаются в tmpfs.
store = Path(os.environ.get("AB_STORE", "/tmp/abbin"))
shutil.rmtree(store, ignore_errors=True)
copies = {"base": [], "candidate": []}
for side, source in (("base", base), ("candidate", candidate)):
    for index in range(int(count)):
        # Одинаковая длина пути у всех копий обеих сторон.
        target = store / f"{side[0]}{index:02d}" / "bsl-cli"
        target.parent.mkdir(parents=True)
        shutil.copy2(source, target)
        copies[side].append(str(target))
pads = list(range(int(count)))
writer = csv.writer(sys.stdout)
writer.writerow(["scenario", "mode", "copy", "side", "ms", "instructions"])
report = []
for name in scenarios:
    for mode in ("plain", "optimize"):
        times = {"base": [], "candidate": []}
        instr = {"base": [], "candidate": []}
        control = None
        for index, pad in enumerate(pads):
            order = ("base", "candidate") if index % 2 == 0 else ("candidate", "base")
            for side in order:
                command = ["perf", "stat", "-x", ";", "-e", "instructions:u", "--", copies[side][pad]]
                if mode == "optimize":
                    command.append("--optimize")
                command.append(str(SCRIPTS / f"{name}.bsl"))
                with tempfile.TemporaryDirectory(dir="/tmp", prefix="ab") as cwd:
                    (Path(cwd) / "benchmarks").mkdir()
                    (Path(cwd) / "scratch").mkdir()
                    done = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
                if done.returncode:
                    sys.exit(f"{name} {mode} {side}: код {done.returncode}\n{done.stderr[-2000:]}")
                lines = done.stdout.strip().splitlines()
                ms = float(lines[-1].replace(",", "."))
                text = re.sub(r"(?<=: )\d+(?:[.,]\d+)?(?= мс)", "<ms>", "\n".join(lines[:-1]))
                if control is None:
                    control = text
                elif text != control:
                    sys.exit(f"{name} {mode}: вывод сторон расходится")
                count_instr = int(next(f.split(";")[0] for f in done.stderr.splitlines() if "instructions:u" in f))
                times[side].append(ms)
                instr[side].append(count_instr)
                writer.writerow([name, mode, pad, side, ms, count_instr])
                sys.stdout.flush()
        b, c = statistics.mean(times["base"]), statistics.mean(times["candidate"])
        report.append((name, mode, b, min(times["base"]), max(times["base"]),
                       c, min(times["candidate"]), max(times["candidate"]),
                       100 * (c / b - 1),
                       100 * (statistics.median(times["candidate"]) / statistics.median(times["base"]) - 1),
                       100 * (statistics.mean(instr["candidate"]) / statistics.mean(instr["base"]) - 1)))
print("# сценарий режим | база: среднее (мин…макс) | кандидат: среднее (мин…макс) | Δсреднего % | Δмедианы % | Δинструкций %", file=sys.stderr)
for r in report:
    print(f"# {r[0]} {r[1]} | {r[2]:.1f} ({r[3]:g}…{r[4]:g}) | {r[5]:.1f} ({r[6]:g}…{r[7]:g}) | {r[8]:+.2f} | {r[9]:+.2f} | {r[10]:+.2f}", file=sys.stderr)
