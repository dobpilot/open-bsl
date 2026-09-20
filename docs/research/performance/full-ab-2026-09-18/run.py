#!/usr/bin/env python3
"""Полный последовательный A/B; результаты и файлы каждого запуска изолированы."""
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
import zipfile

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
SNAPSHOTS = ROOT / "target/flamegraphs/after-allocations-2026-09-18"
WORK = ROOT / "target/full-ab-2026-09-18"
if len(sys.argv) > 1:
    OUT = OUT / sys.argv[1]
    WORK = WORK / sys.argv[1]
SOURCE = SNAPSHOTS / "current-src/benchmarks"
UUID = re.compile(rb"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}")


def normalized_output(stdout):
    lines = stdout.strip().splitlines()
    if not lines or not re.fullmatch(r"\d+(?:[.,]\d+)?", lines[-1]):
        raise ValueError("нет числовой последней строки времени")
    control = re.sub(r"(?<=: )\d+(?:[.,]\d+)?(?= мс)", "<ms>", "\n".join(lines[:-1]))
    if "РАЗОШЛОСЬ" in control:
        raise ValueError("сценарий сообщил РАЗОШЛОСЬ")
    return float(lines[-1].replace(",", ".")), control


def digest_file(path):
    h = hashlib.sha256()
    if path.suffix in (".zip", ".xlsx"):
        # ZIP-время и упаковка не являются содержимым документа.
        with zipfile.ZipFile(path) as archive:
            for entry in sorted(archive.infolist(), key=lambda e: e.filename):
                h.update(entry.filename.encode())
                h.update(b"\0")
                h.update(archive.read(entry))
    elif path.name == "edata_writer.xml":
        # Случайные UUID нумеруются по первому появлению; связи сохраняются.
        names = {}
        def canonical(match):
            value = match.group().lower()
            if value not in names:
                names[value] = f"UUID-{len(names)}".encode()
            return names[value]
        h.update(UUID.sub(canonical, path.read_bytes()))
    else:
        with path.open("rb") as stream:
            while chunk := stream.read(1024 * 1024):
                h.update(chunk)
    return h.hexdigest()


def invoke(command, cwd):
    start = time.monotonic()
    process = subprocess.Popen(command, cwd=cwd, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=1200)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
        return -1, stdout, stderr + "\nTIMEOUT 1200s", time.monotonic() - start
    return process.returncode, stdout, stderr, time.monotonic() - start


def main():
    WORK.mkdir(parents=True, exist_ok=False)
    OUT.mkdir(parents=True, exist_ok=True)
    (WORK / "scripts").mkdir()
    (WORK / "scripts/modules").symlink_to(SOURCE / "modules", target_is_directory=True)
    (WORK / "crates").symlink_to(SNAPSHOTS / "current-src/crates", target_is_directory=True)
    (OUT / "logs").mkdir(exist_ok=True)
    rows = OUT / "runs.csv"
    failures = []
    references = {}
    with rows.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=["scenario", "mode", "pair", "side",
            "status", "elapsed_ms", "wall_seconds", "instructions", "cycles", "control_sha256", "files_sha256"])
        writer.writeheader()
        for script in sorted(SOURCE.glob("*.bsl")):
            name = script.stem
            if os.getenv("AB_ONLY") and name not in os.environ["AB_ONLY"].split(","):
                continue
            heavy = name.startswith("csv_write") or name in (
                "background_jobs", "table_compare", "table_compare2", "table_save_load")
            pairs = 3 if heavy else 5
            # Только перенаправление путей: алгоритм и масштабы не меняются.
            prepared = WORK / "scripts" / script.name
            source = script.read_text().replace('"benchmarks/data/', '"' + str(SOURCE / "data") + '/')
            prepared.write_text(source.replace('"/tmp/', '"scratch/'))
            for mode in ("plain", "optimize"):
                disabled = set()
                for pair in range(1, pairs + 1):
                    for side in (("main", "current") if pair % 2 else ("current", "main")):
                        if side in disabled:
                            continue
                        label = f"{name}-{mode}-{pair}-{side}"
                        cwd = WORK / label
                        (cwd / "benchmarks").mkdir(parents=True)
                        (cwd / "scratch").mkdir()
                        binary = SNAPSHOTS / (side + "-src") / "target/release/bsl-cli"
                        command = ["perf", "stat", "-x", ";", "-e", "instructions:u,cycles:u", "--", str(binary)]
                        if mode == "optimize":
                            command.append("--optimize")
                        command.append(str(prepared))
                        print(f"RUN {label}", flush=True)
                        code, stdout, stderr, wall = invoke(command, cwd)
                        (OUT / "logs" / (label + ".out")).write_text(stdout)
                        (OUT / "logs" / (label + ".err")).write_text(stderr)
                        row = dict(scenario=name, mode=mode, pair=pair, side=side,
                                   status="ok", wall_seconds=round(wall, 6))
                        try:
                            if code:
                                raise ValueError(f"exit={code}")
                            ms, control = normalized_output(stdout)
                            counts = {}
                            for fields in csv.reader(stderr.splitlines(), delimiter=";"):
                                if len(fields) > 2 and fields[2] in ("instructions:u", "cycles:u"):
                                    if fields[2] in counts:
                                        raise ValueError("повторный счётчик perf")
                                    counts[fields[2]] = int(fields[0])
                            files = {str(f.relative_to(cwd)): digest_file(f)
                                     for f in sorted(cwd.rglob("*")) if f.is_file()}
                            if name.startswith("csv_write") and "test.csv" not in files:
                                raise ValueError("не создан test.csv")
                            required = {
                                "edata_writer": ["benchmarks/edata_writer.xml"],
                                "invoice_doc_generator": ["benchmarks/invoice_doc.mxl", "benchmarks/invoice_doc.pdf", "benchmarks/invoice_doc.xlsx"],
                                "textdoc_invoice": ["benchmarks/textdoc_invoice.txt"],
                                "table_save_load": ["scratch/open-bsl-bench-vt-save.txt"],
                                "zip_write": ["scratch/open-bsl-zip-write-out.zip"],
                            }
                            if any(path not in files for path in required.get(name, [])):
                                raise ValueError("не созданы обязательные выходные файлы")
                            if name == "zip_read" and not any(path.startswith("scratch/open-bsl-zip-read-out/") for path in files):
                                raise ValueError("архив не распакован")
                            signature = (control, files)
                            if name not in references:
                                references[name] = signature
                            elif signature != references[name]:
                                row["status"] = "mismatch"
                                failures.append({"run": label, "reason": "control/files mismatch"})
                            row.update(elapsed_ms=ms, instructions=counts["instructions:u"],
                                       cycles=counts["cycles:u"],
                                       control_sha256=hashlib.sha256(control.encode()).hexdigest(),
                                       files_sha256=hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest())
                            (OUT / "logs" / (label + ".files.json")).write_text(json.dumps(files, indent=2))
                        except (ValueError, KeyError, OSError, zipfile.BadZipFile) as error:
                            row["status"] = "failed"
                            failures.append({"run": label, "reason": str(error)})
                            disabled.add(side)
                        writer.writerow(row)
                        stream.flush()
                        print(f"DONE {label}: {row['status']} {row.get('elapsed_ms', '-')} ms", flush=True)
    (OUT / "failures.json").write_text(json.dumps(failures, ensure_ascii=False, indent=2))
    print(f"FINISHED; failures/mismatches: {len(failures)}", flush=True)
    return bool(failures)


if __name__ == "__main__":
    sys.exit(main())
