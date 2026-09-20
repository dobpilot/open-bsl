#!/usr/bin/env python3
"""Проверяет полноту пар; не превращает отказ или расхождение в ускорение."""
import csv
from pathlib import Path
import statistics

OUT = Path(__file__).resolve().parent
rows = list(csv.DictReader((OUT / "runs.csv").open()))
retry = OUT / "retry/runs.csv"
if retry.exists():
    replacements = list(csv.DictReader(retry.open()))
    names = {r["scenario"] for r in replacements}
    rows = [r for r in rows if r["scenario"] not in names] + replacements

with (OUT / "final-runs.csv").open("w", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=rows[0].keys())
    writer.writeheader()
    writer.writerows(rows)

summary = []
for name in sorted({r["scenario"] for r in rows}):
    heavy = name.startswith("csv_write") or name in ("background_jobs", "table_compare", "table_compare2", "table_save_load")
    expected = 3 if heavy else 5
    for mode in ("plain", "optimize"):
        selected = [r for r in rows if r["scenario"] == name and r["mode"] == mode]
        pairs = {(int(r["pair"]), r["side"]): r for r in selected}
        complete = len(selected) == 2 * expected and len(pairs) == len(selected)
        complete = complete and all((p, s) in pairs for p in range(1, expected + 1) for s in ("main", "current"))
        status = "ok" if complete and all(r["status"] == "ok" for r in selected) else "failed/incomplete"
        record = dict(scenario=name, mode=mode, status=status, runs=len(selected))
        if status == "ok":
            for side in ("main", "current"):
                record[side + "_median_ms"] = statistics.median(float(r["elapsed_ms"]) for r in selected if r["side"] == side)
            for field in ("elapsed_ms", "instructions", "cycles"):
                ratios = []
                for pair in range(1, expected + 1):
                    baseline = float(pairs[pair, "main"][field])
                    candidate = float(pairs[pair, "current"][field])
                    if baseline == 0:
                        break
                    ratios.append(100 * (candidate / baseline - 1))
                if len(ratios) == expected:
                    record[field + "_delta_mean"] = statistics.mean(ratios)
                    record[field + "_delta_min"] = min(ratios)
                    record[field + "_delta_max"] = max(ratios)
        summary.append(record)

fields = list(dict.fromkeys(k for row in summary for k in row))
with (OUT / "summary.csv").open("w", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=fields)
    writer.writeheader()
    writer.writerows(summary)

with (OUT / "results.md").open("w") as f:
    f.write("# Полный A/B: результаты\n\nДельта — среднее попарное current/main − 1. "
            "Интервал — полный диапазон пар, не доверительный интервал.\n\n")
    for mode in ("plain", "optimize"):
        f.write(f"## {mode}\n\n| Сценарий | main, мс | current, мс | Время, % (диапазон) | Инструкции, % |\n|---|---:|---:|---:|---:|\n")
        for r in summary:
            if r["mode"] != mode:
                continue
            if r["status"] != "ok":
                f.write(f"| {r['scenario']} | — | — | ошибка/неполный прогон | — |\n")
                continue
            delta = "—"
            if "elapsed_ms_delta_mean" in r:
                delta = f"{r['elapsed_ms_delta_mean']:+.2f} ({r['elapsed_ms_delta_min']:+.2f}…{r['elapsed_ms_delta_max']:+.2f})"
            f.write(f"| {r['scenario']} | {r['main_median_ms']:g} | {r['current_median_ms']:g} | {delta} | {r.get('instructions_delta_mean', 0):+.2f} |\n")
        f.write("\n")
print(f"scenarios={len(summary)//2}, successful_modes={sum(r['status']=='ok' for r in summary)}/{len(summary)}, runs={len(rows)}")
