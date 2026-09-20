#!/usr/bin/env python3
"""Последовательные замеры двойников с сохранением вывода и кодов возврата."""
import csv
import json
import os
from pathlib import Path
import shutil
import sys
import run

OUT = run.OUT / "twins"
WORK = run.ROOT / "target/full-ab-2026-09-20-twins"
if len(sys.argv) > 1:
    OUT = OUT / sys.argv[1]
    WORK = WORK / sys.argv[1]


def main():
    WORK.mkdir(exist_ok=False)
    OUT.mkdir(exist_ok=False)
    scripts = WORK / "scripts"
    scripts.mkdir()
    (scripts / "benchmark_data.py").symlink_to(run.SOURCE / "benchmark_data.py")
    for folder in ("lib", "modules"):
        (scripts / folder).symlink_to(run.SOURCE / folder, target_is_directory=True)
    (WORK / "crates").symlink_to(run.SNAPSHOTS / "current-src/crates", target_is_directory=True)
    runtimes = {"lua": shutil.which("lua"), "luajit": shutil.which("luajit"),
                "python": shutil.which("python3"), "oscript": shutil.which("oscript")}
    if not runtimes["oscript"] and Path("/opt/oscript/bin/oscript").exists():
        runtimes["oscript"] = "/opt/oscript/bin/oscript"
    (OUT / "runtimes.json").write_text(json.dumps(runtimes, indent=2))
    with (OUT / "runs.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=["scenario", "runtime", "run", "status", "elapsed_ms", "wall_seconds"])
        writer.writeheader()
        for bsl in sorted(run.SOURCE.glob("*.bsl")):
            name = bsl.stem
            if os.getenv("AB_ONLY") and name not in os.environ["AB_ONLY"].split(","):
                continue
            heavy = name.startswith("csv_write") or name in ("background_jobs", "table_compare", "table_compare2", "table_save_load")
            for runtime, binary in runtimes.items():
                suffix = {"lua": ".lua", "luajit": ".lua", "python": ".py", "oscript": ".bsl"}[runtime]
                source = bsl.with_suffix(suffix)
                if not binary or not source.exists():
                    writer.writerow(dict(scenario=name, runtime=runtime, status="unavailable"))
                    stream.flush()
                    continue
                prepared = scripts / source.name
                prepared.write_text(source.read_text().replace("benchmarks/data/", str(run.SOURCE / "data") + "/").replace("/tmp/", "scratch/"))
                for number in range(1, (3 if heavy else 5) + 1):
                    label = f"{name}-{runtime}-{number}"
                    cwd = WORK / label
                    (cwd / "scratch").mkdir(parents=True)
                    (cwd / "benchmarks").mkdir()
                    print(f"RUN {label}", flush=True)
                    code, stdout, stderr, wall = run.invoke([binary, str(prepared)], cwd)
                    (OUT / (label + ".out")).write_text(stdout)
                    (OUT / (label + ".err")).write_text(stderr)
                    row = dict(scenario=name, runtime=runtime, run=number, status="ok", wall_seconds=wall)
                    try:
                        if code:
                            raise ValueError(f"exit={code}")
                        row["elapsed_ms"], _ = run.normalized_output(stdout)
                        if name.startswith("csv_write"):
                            expected = run.WORK / f"{name}-plain-1-current/test.csv"
                            if run.digest_file(cwd / "test.csv") != run.digest_file(expected):
                                raise ValueError("CSV отличается от current")
                    except (ValueError, OSError) as error:
                        row["status"] = str(error)
                    writer.writerow(row)
                    stream.flush()
                    print(f"DONE {label}: {row['status']}", flush=True)
                    if row["status"] != "ok":
                        break


if __name__ == "__main__":
    main()
