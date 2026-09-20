"""Чередующийся A/B до и после преобразования файлового буфера на месте."""
import csv
import hashlib
from pathlib import Path
import statistics
import re
import subprocess

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
BINS = {
    "before": ROOT / "target/table-number-write-after/bsl-cli",
    "after": ROOT / "target/table-file-buffer-after/bsl-cli",
}

if __name__ == "__main__":
    (OUT / "binaries.sha256").write_text("".join(
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n"
        for path in BINS.values()
    ))
    rows = []
    controls = {}
    with (OUT / "ab.csv").open("w") as stream:
        writer = csv.DictWriter(stream, fieldnames=["scenario", "mode", "pair", "side", "ms", "instructions"])
        writer.writeheader()
        for scenario in ("table_compare", "table_compare2", "table_save_load"):
            for mode in ("plain", "optimize"):
                for pair in range(1, 4):
                    for side in (("before", "after") if pair % 2 else ("after", "before")):
                        command = ["perf", "stat", "-x", ";", "-e", "instructions:u", "--", str(BINS[side])]
                        if mode == "optimize":
                            command.append("--optimize")
                        command.append(str(ROOT / "target/table-file-buffer-after" / f"{scenario}.bsl"))
                        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=300, check=True)
                        label = f"{scenario}-{mode}-{pair}-{side}"
                        (OUT / f"{label}.txt").write_text(result.stdout + result.stderr)
                        lines = result.stdout.strip().splitlines()
                        control = re.sub(r"(?<=: )\d+(?:[.,]\d+)?(?= мс)", "<ms>", "\n".join(lines[:-1]))
                        assert "РАЗОШЛОСЬ" not in control, label
                        if scenario == "table_save_load":
                            saved = ROOT / "target/table-file-buffer-after/save.txt"
                            control += hashlib.sha256(saved.read_bytes()).hexdigest()
                        if scenario in controls:
                            assert controls[scenario] == control, label
                        controls[scenario] = control
                        counts = [int(fields[0]) for fields in csv.reader(result.stderr.splitlines(), delimiter=";")
                                  if len(fields) > 2 and fields[2] == "instructions:u"]
                        assert len(counts) == 1, label
                        row = dict(scenario=scenario, mode=mode, pair=pair, side=side,
                                   ms=float(lines[-1].replace(",", ".")), instructions=counts[0])
                        rows.append(row)
                        writer.writerow(row)
                        stream.flush()
                        print(label, row["ms"], flush=True)
    assert len(rows) == 36
    for scenario in controls:
        for mode in ("plain", "optimize"):
            selected = [r for r in rows if r["scenario"] == scenario and r["mode"] == mode]
            pairs = {(r["pair"], r["side"]): r for r in selected}
            for field in ("ms", "instructions"):
                deltas = [100 * (pairs[p, "after"][field] / pairs[p, "before"][field] - 1) for p in range(1, 4)]
                print(scenario, mode, field, statistics.mean(deltas), min(deltas), max(deltas))
