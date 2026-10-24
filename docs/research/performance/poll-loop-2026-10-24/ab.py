#!/usr/bin/env python3
"""Чередующийся A/B двух бинарников bsl-cli по выбранным сценариям.

Использование: ab.py BASE CANDIDATE PAIRS СЦЕНАРИЙ... > результат.csv
Сценарии берутся подготовленными драйвером полного A/B 23 октября (пути
данных и вывода уже перенаправлены). На каждую пару сначала идёт то база,
то кандидат; вывод без времени сверяется между сторонами.
"""
import csv, re, statistics, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SCRIPTS = ROOT / "target/full-ab-2026-10-23/scripts"
base, candidate, pairs, *scenarios = sys.argv[1:]
base, candidate = str(Path(base).resolve()), str(Path(candidate).resolve())
writer = csv.writer(sys.stdout)
writer.writerow(["scenario", "mode", "pair", "side", "ms", "instructions"])
summary = []
for name in scenarios:
    for mode in ("plain", "optimize"):
        rows = {"base": [], "candidate": []}
        control = {}
        for pair in range(1, int(pairs) + 1):
            order = ("base", "candidate") if pair % 2 else ("candidate", "base")
            for side in order:
                binary = base if side == "base" else candidate
                command = ["perf", "stat", "-x", ";", "-e", "instructions:u", "--", binary]
                if mode == "optimize":
                    command.append("--optimize")
                command.append(str(SCRIPTS / f"{name}.bsl"))
                with tempfile.TemporaryDirectory() as cwd:
                    (Path(cwd) / "benchmarks").mkdir()
                    (Path(cwd) / "scratch").mkdir()
                    done = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
                if done.returncode:
                    sys.exit(f"{name} {mode} {side}: код {done.returncode}\n{done.stderr[-2000:]}")
                lines = done.stdout.strip().splitlines()
                ms = float(lines[-1].replace(",", "."))
                text = re.sub(r"(?<=: )\d+(?:[.,]\d+)?(?= мс)", "<ms>", "\n".join(lines[:-1]))
                if control.setdefault(side, text) != text or control.get("base", text) != text:
                    sys.exit(f"{name} {mode}: вывод сторон расходится")
                instructions = int(next(f.split(";")[0] for f in done.stderr.splitlines() if "instructions:u" in f))
                rows[side].append((ms, instructions))
                writer.writerow([name, mode, pair, side, ms, instructions])
                sys.stdout.flush()
        ratios = [c[0] / b[0] - 1 for b, c in zip(rows["base"], rows["candidate"]) if b[0]]
        instr = [c[1] / b[1] - 1 for b, c in zip(rows["base"], rows["candidate"])]
        summary.append((name, mode, statistics.median(r[0] for r in rows["base"]),
                        statistics.median(r[0] for r in rows["candidate"]),
                        100 * statistics.mean(ratios), 100 * min(ratios), 100 * max(ratios),
                        100 * statistics.mean(instr)))
print("# сценарий режим база_мс кандидат_мс Δвремя% (диапазон) Δинструкции%", file=sys.stderr)
for s in summary:
    print(f"# {s[0]} {s[1]} {s[2]:g} {s[3]:g} {s[4]:+.2f} ({s[5]:+.2f}…{s[6]:+.2f}) {s[7]:+.2f}", file=sys.stderr)
