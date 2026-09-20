#!/usr/bin/env python3
"""Native-замер на localhost/test с сохранением и проверкой восстановления CF/XML.

Не запускает службы. Требует исключить параллельное редактирование конфигурации.
Артефакты не удаляются, oracle репозитория автоматически не меняется.
SIGKILL не перехватывается: после него восстановление выполняют вручную из CF.
"""

import argparse
import os
from pathlib import Path
import pwd
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
from xml.dom import minidom


HERE = Path(__file__).resolve().parent
CONNECTION = "localhost/test"


def xml_snapshot(root):
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(f"Ссылка в XML-выгрузке: {path}")
        relative = path.relative_to(root)
        if path.is_file() and relative != Path("ConfigDumpInfo.xml"):
            result[str(relative)] = path.read_bytes()
    if not result.get("Configuration.xml"):
        raise RuntimeError(f"Нет непустого Configuration.xml: {root}")
    return result


def prepare_configuration(source, output, script, result_path, observe_client_errors=False):
    shutil.copytree(source, output)
    shutil.copytree(HERE / "cfg-src", output, dirs_exist_ok=True)
    output.chmod(0o700)
    config = output / "Configuration.xml"
    document = minidom.parse(str(config))
    configurations = [node for node in document.documentElement.childNodes
                      if node.nodeType == node.ELEMENT_NODE and node.localName == "Configuration"]
    if len(configurations) != 1:
        raise RuntimeError("Ожидался один объект Configuration")
    children = [node for node in configurations[0].childNodes
                if node.nodeType == node.ELEMENT_NODE and node.localName == "ChildObjects"]
    if len(children) != 1:
        raise RuntimeError("Ожидался один ChildObjects конфигурации")
    children = children[0]
    existing = [node for node in children.childNodes
                if node.nodeType == node.ELEMENT_NODE and node.localName == "DataProcessor"
                and "".join(part.data for part in node.childNodes
                            if part.nodeType == part.TEXT_NODE).strip() == "Замеры"]
    if not existing:
        prefix = f"{children.prefix}:" if children.prefix else ""
        node = document.createElementNS(children.namespaceURI, prefix + "DataProcessor")
        node.appendChild(document.createTextNode("Замеры"))
        children.appendChild(node)
        config.write_bytes(document.toxml(encoding="utf-8"))
    module = output / "DataProcessors/Замеры/Forms/Форма/Ext/Form/Module.bsl"
    module.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.executable, str(HERE / "gen-form-module.py"), str(script),
                    str(module), str(result_path)], check=True, stdout=subprocess.DEVNULL)
    if observe_client_errors:
        application = output / "Ext/ManagedApplicationModule.bsl"
        escaped_path = str(result_path).replace('"', '""')
        observer = f'''
Процедура ОбработкаОтображенияОшибки(ИнформацияОшибкиПробы, ТребуетсяЗавершениеСеанса, СтандартнаяОбработка)
    СтандартнаяОбработка = Ложь;
    ТекстОшибкиПробы = ИнформацияОшибкиПробы.Описание;
    ТекстОшибкиПробы = СтрЗаменить(ТекстОшибкиПробы, Символ(13), "\\r");
    ТекстОшибкиПробы = СтрЗаменить(ТекстОшибкиПробы, Символ(10), "\\n");
    ВыводОшибкиПробы = Новый ЗаписьТекста("{escaped_path}", , , Истина);
    ВыводОшибкиПробы.ЗаписатьСтроку("client.error" + Символ(9) + ТекстОшибкиПробы);
    ВыводОшибкиПробы.Закрыть();
КонецПроцедуры
'''
        application.write_text(application.read_text(encoding="utf-8") + observer,
                               encoding="utf-8")


def validate_output(path, marker):
    text = path.read_text(encoding="utf-8-sig")
    lines = text.splitlines()
    begin = f"file.begin\t{marker}"
    end = f"file.end\t{marker}"
    if ([line for line in lines if line.startswith("file.begin\t")] != [begin]
            or [line for line in lines if line.startswith("file.end\t")] != [end]
            or lines.index(begin) >= lines.index(end)):
        raise RuntimeError("Нет единственных упорядоченных маркеров ожидаемой пробы")
    if any(line.startswith(("file.setup_error\t", "file.cleanup_error\t")) for line in lines):
        raise RuntimeError("Ошибка подготовки или очистки измерителя")
    return "\n".join(lines) + "\n"


def observe_temporary_files(output, work):
    paths = {}
    for line in output.splitlines():
        if not line.startswith("lifetime.path."):
            continue
        key, path = line.split("\t", 1)
        role = key.removeprefix("lifetime.path.")
        candidate = Path(path)
        if (role in paths or role not in {"closed", "released", "open", "name"}
                or candidate.parent != Path("/tmp") or not candidate.name.startswith("v8_")):
            raise RuntimeError("Некорректный путь или роль наблюдаемого временного файла")
        paths[role] = candidate
    if len(paths) != 4:
        raise RuntimeError("Для наблюдения нужны четыре пути временных файлов")
    lines = ["phase\tcase\tstate\tinode\tsize"]
    for role, path in paths.items():
        try:
            metadata = path.lstat()
        except FileNotFoundError:
            lines.append(f"after_enterprise\t{role}\tabsent\t-\t-")
        else:
            kind = "file" if stat.S_ISREG(metadata.st_mode) else "other"
            lines.append(f"after_enterprise\t{role}\t{kind}\t{metadata.st_ino}\t{metadata.st_size}")
    (work / "host-after-enterprise.tsv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def measure(script, marker, work, native, observe_temp_lifetime=False, observe_client_errors=False):
    """native(label, mode, *arguments) подменяется только в offline-тестах."""
    original_cf = work / "original.cf"
    original_xml = work / "original-xml"
    original_xml.mkdir(mode=0o700)
    native("backup-cf", "DESIGNER", "/DumpCfg", str(original_cf))
    if not original_cf.is_file() or original_cf.stat().st_size == 0:
        raise RuntimeError("Не создана непустая резервная копия CF; база не менялась")
    original_cf.chmod(0o600)
    native("backup-xml", "DESIGNER", "/DumpConfigToFiles", str(original_xml))
    original = xml_snapshot(original_xml)
    probe = work / "probe-xml"
    raw = work / "io/platform.raw.txt"
    prepare_configuration(original_xml, probe, script, raw, observe_client_errors)
    before = work / "before-load-xml"
    before.mkdir(mode=0o700)
    native("before-load", "DESIGNER", "/DumpConfigToFiles", str(before))
    if xml_snapshot(before) != original:
        raise RuntimeError("Конфигурация изменилась во время подготовки; база не менялась runner")

    # finally начинается ДО загрузки: её ошибка не доказывает отсутствие изменений.
    try:
        native("load", "DESIGNER", "/LoadConfigFromFiles", str(probe))
        native("check", "DESIGNER", "/CheckModules", "-ThinClient", "-Server")
        native("update", "DESIGNER", "/UpdateDBCfg")
        native("run", "ENTERPRISE")
        output = validate_output(raw, marker)
        if observe_temp_lifetime:
            observe_temporary_files(output, work)
    finally:
        # Повторный сигнал не должен оборвать уже начатое восстановление.
        handlers = {sig: signal.signal(sig, signal.SIG_IGN)
                    for sig in (signal.SIGINT, signal.SIGTERM)}
        try:
            native("restore-cf", "DESIGNER", "/LoadCfg", str(original_cf))
            native("restore-update", "DESIGNER", "/UpdateDBCfg")
            restored = work / "restored-xml"
            restored.mkdir(mode=0o700)
            native("restore-xml", "DESIGNER", "/DumpConfigToFiles", str(restored))
            if xml_snapshot(restored) != original:
                raise RuntimeError("Восстановленный XML отличается от исходного")
            (work / "restoration.txt").write_text(
                "CF восстановлен; XML совпал, кроме корневого ConfigDumpInfo.xml.\n",
                encoding="utf-8")
        except BaseException as error:
            raise RuntimeError(f"ВОССТАНОВЛЕНИЕ НЕ ПОДТВЕРЖДЕНО; CF и логи: {work}") from error
        finally:
            for sig, handler in handlers.items():
                signal.signal(sig, handler)
    (work / "platform.txt").write_text(output, encoding="utf-8")
    print(f"RESTORED: XML совпал; результат {work / 'platform.txt'}", flush=True)


def run_native(platform, timeout, work, label, mode, *arguments, cwd=None):
    environment = os.environ.copy()
    environment.pop("WAYLAND_DISPLAY", None)
    environment["GDK_BACKEND"] = "x11"
    shim = os.environ.get("ONEC_SHIM", str(Path.home() / ".local/lib/1c-wk41"))
    environment["LD_LIBRARY_PATH"] = f"{shim}:/usr/lib"
    command = [platform, mode, "/S", CONNECTION, *arguments, "/DisableStartupDialogs",
               "/DisableStartupMessages", "/Out", str(work / f"{label}.log")]
    with (work / f"{label}.process.log").open("wb") as log:
        process = subprocess.Popen(command, env=environment, stdout=log, stderr=log,
                                   cwd=cwd, start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
        except BaseException:
            handlers = {sig: signal.signal(sig, signal.SIG_IGN)
                        for sig in (signal.SIGINT, signal.SIGTERM)}
            try:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    pass
                # Лидер мог выйти раньше своих детей; завершаем остаток именно
                # этой группы перед попыткой восстановления конфигурации.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
            finally:
                for sig, handler in handlers.items():
                    signal.signal(sig, handler)
            raise
        if code:
            raise RuntimeError(f"1cv8: {label}, код {code}; лог {work / (label + '.log')}")


def prepare_io(work, server_uid):
    """Серверу доступен только I/O-каталог, не CF и выгрузки конфигурации."""
    io_root = work / "io"
    io_root.mkdir(mode=0o700)
    if server_uid != os.getuid():
        subprocess.run(["setfacl", "-m", f"u:{server_uid}:x", str(work)], check=True)
        subprocess.run(["setfacl", "-m",
                        f"u:{server_uid}:rwx,d:u:{server_uid}:rwx,d:u:{os.getuid()}:rwx",
                        str(io_root)], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("script", type=Path)
    parser.add_argument("--marker", required=True, help="Значение file.begin/file.end")
    parser.add_argument("--allow-config-mutation", action="store_true")
    parser.add_argument("--observe-client-errors", action="store_true",
                        help="Записывать ошибки приложения вместо модального отображения")
    parser.add_argument("--platform", default=os.environ.get(
        "ONEC_PLATFORM", "/opt/1cv8/x86_64/8.3.27.2342/1cv8"))
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--observe-temp-lifetime", action="store_true",
                        help="Снимок четырёх путей file-temp-return до восстановления CF")
    parser.add_argument("--enterprise-cwd-in-io", action="store_true",
                        help="Запустить только ENTERPRISE из приватного io/enterprise-cwd")
    parser.add_argument("--server-user", default="usr1cv8",
                        help="Пользователь серверного процесса; ACL только для I/O пробы")
    options = parser.parse_args()
    if not options.allow_config_mutation:
        parser.error("Требуется --allow-config-mutation для изменения localhost/test")
    if not options.script.is_file() or options.timeout <= 0:
        parser.error("Нужны существующий скрипт и положительный timeout")
    if not os.environ.get("DISPLAY"):
        parser.error("Нужен DISPLAY для native-клиента 1С")
    if not os.path.isfile(options.platform) or not os.access(options.platform, os.X_OK):
        parser.error("Не найден исполняемый 1cv8")
    try:
        server_uid = pwd.getpwnam(options.server_user).pw_uid
    except KeyError:
        parser.error("Неизвестный пользователь серверного процесса")
    os.umask(0o077)
    work = Path(tempfile.mkdtemp(prefix="open-bsl-server-measure-", dir="/tmp"))
    print(f"Артефакты: {work}", flush=True)

    def interrupted(signum, _frame):
        raise InterruptedError(f"Получен сигнал {signum}")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        prepare_io(work, server_uid)
        enterprise_cwd = None
        if options.enterprise_cwd_in_io:
            enterprise_cwd = work / "io/enterprise-cwd"
            enterprise_cwd.mkdir(mode=0o700)
        measure(options.script.resolve(), options.marker, work,
                lambda label, mode, *args: run_native(
                    options.platform, options.timeout, work, label, mode, *args,
                    cwd=enterprise_cwd if label == "run" else None),
                observe_temp_lifetime=options.observe_temp_lifetime,
                observe_client_errors=options.observe_client_errors)
    except (Exception, KeyboardInterrupt) as error:
        print(f"Замер не выполнен: {error}; артефакты: {work}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
