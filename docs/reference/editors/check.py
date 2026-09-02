#!/usr/bin/env python3
"""Проверка отладчика open-bsl подставным клиентом DAP.

Повторяет порядок запросов, который шлёт редактор: `initialize`,
`setBreakpoints`, `configurationDone`, затем осмотр остановки. Редактор
для этого не нужен — если проверка проходит, а в редакторе не работает,
дело в его конфигурации, а не в адаптере, и это как раз то разделение,
ради которого она написана.

    python3 docs/reference/editors/check.py [путь-к-bsl-cli]
"""

import json
import re
import socket
import subprocess
import sys
import tempfile
from pathlib import Path

# Счётчик объявлен `Перем`: без этого `Счёт` внутри процедуры — НОВАЯ
# локальная, `Неопределено + 1` роняет прогон на первом же вызове, и
# проверка показывает одну остановку вместо трёх. Ошибка выглядит как
# дефект отладчика, хотя дефект в скрипте.
SCRIPT = """Перем Счёт;

Процедура Считать()
    ш = 1;
    Счёт = Счёт + ш;
КонецПроцедуры

Счёт = 0;
Для н = 1 По 3 Цикл
    Считать();
КонецЦикла;
Сообщить(Счёт);
"""

# Строка `Счёт = Счёт + ш;` — тело процедуры, вызываемой трижды.
BREAKPOINT = 5


class Client:
    def __init__(self, cli: str, script: Path):
        self.proc = subprocess.Popen(
            [cli, "--debug", "--debug-port", "0", str(script)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        first = self.proc.stderr.readline().decode()
        if "ждёт подключения" not in first:
            raise SystemExit(f"адаптер не начал слушать: {first!r}")
        port = int(first.strip().rsplit(":", 1)[1])
        self.sock = socket.create_connection(("127.0.0.1", port))
        self.sock.settimeout(20)
        self.seq = 0
        self.buf = b""

    def send(self, command, **kw):
        self.seq += 1
        body = json.dumps(
            {"seq": self.seq, "type": "request", "command": command, **kw}
        ).encode()
        self.sock.sendall(
            f"Content-Length: {len(body)}\r\n\r\n".encode() + body
        )

    def read(self):
        while True:
            i = self.buf.find(b"\r\n\r\n")
            if i >= 0:
                n = int(re.search(rb"Content-Length: (\d+)", self.buf[:i]).group(1))
                if len(self.buf) >= i + 4 + n:
                    message = json.loads(self.buf[i + 4 : i + 4 + n])
                    self.buf = self.buf[i + 4 + n :]
                    return message
            chunk = self.sock.recv(4096)
            if not chunk:
                return None
            self.buf += chunk

    def close(self):
        self.proc.kill()


def main() -> int:
    cli = sys.argv[1] if len(sys.argv) > 1 else "./target/debug/bsl-cli"
    tmp = Path(tempfile.mkdtemp())
    script = tmp / "проверка.bsl"
    script.write_text(SCRIPT, encoding="utf-8")

    c = Client(cli, script)
    # Ровно то, что шлёт редактор, и в том же порядке.
    c.send("initialize", arguments={"adapterID": "open-bsl", "linesStartAt1": True})
    c.send(
        "setBreakpoints",
        arguments={"source": {"path": str(script)}, "breakpoints": [{"line": BREAKPOINT}]},
    )
    c.send("configurationDone")

    checks = []
    stops = 0
    asked = False
    while True:
        m = c.read()
        if m is None:
            break
        if m.get("command") == "setBreakpoints":
            bp = m["body"]["breakpoints"][0]
            checks.append(("точка подтверждена", bp.get("verified") is True))
            checks.append((f"подтверждена на строке {BREAKPOINT}", bp.get("line") == BREAKPOINT))
        elif m.get("event") == "stopped":
            stops += 1
            if not asked:
                asked = True
                c.send("stackTrace", arguments={"threadId": 1})
            else:
                c.send("continue", arguments={"threadId": 1})
        elif m.get("command") == "stackTrace":
            frames = m["body"]["stackFrames"]
            checks.append((f"остановка на строке {BREAKPOINT}", frames[0]["line"] == BREAKPOINT))
            checks.append(("кадров больше одного", len(frames) > 1))
            checks.append(
                ("у кадра есть файл", bool(frames[0].get("source", {}).get("path")))
            )
            checks.append(
                (
                    "путь абсолютный",
                    str(frames[0]["source"]["path"]).startswith("/"),
                )
            )
            c.send("scopes", arguments={"frameId": 0})
        elif m.get("command") == "scopes":
            ref = m["body"]["scopes"][0]["variablesReference"]
            c.send("variables", arguments={"variablesReference": ref})
        elif m.get("command") == "variables":
            names = {v["name"]: v["value"] for v in m["body"]["variables"]}
            checks.append(("видна локальная ш = 1", names.get("ш") == "1"))
            c.send("evaluate", arguments={"expression": "ш + 1", "frameId": 0})
        elif m.get("command") == "evaluate":
            checks.append(("вычисление в кадре даёт 2", m["body"]["result"] == "2"))
            c.send("continue", arguments={"threadId": 1})
        elif m.get("event") == "output":
            checks.append(("вывод продублирован событием", True))
        elif m.get("event") == "terminated":
            break
    checks.append((f"остановок по числу витков — три (было {stops})", stops == 3))
    c.close()

    width = max(len(name) for name, _ in checks)
    ok = True
    for name, passed in checks:
        print(f"  {name:<{width}}  {'да' if passed else 'НЕТ'}")
        ok &= passed
    print("\nадаптер готов к подключению редактора" if ok else "\nадаптер отвечает не так, как ждёт редактор")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
