"""Offline-проверки сохранности серверного runner; настоящую 1С не запускают."""

import importlib.util
from contextlib import ExitStack
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch


SCRIPT = Path(__file__).with_name("run-on-server-1c.py")
SPEC = importlib.util.spec_from_file_location("server_runner", SCRIPT)
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)

CONFIG = b'<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses"><Configuration><ChildObjects/></Configuration></MetaDataObject>'
OUTPUT = "\ufefffile.begin\tprobe\r\nvalue\t7\r\nfile.end\tprobe\r\n"


class Native:
    def __init__(self, work, fail=None, output=OUTPUT, drift=None):
        self.work = work
        self.fail = fail
        self.output = output
        self.drift = drift
        self.calls = []

    def __call__(self, label, mode, *arguments):
        if (self.work / "platform.txt").exists():
            raise AssertionError("Результат опубликован до проверки восстановления")
        self.calls.append(label)
        if label == self.fail:
            raise RuntimeError(f"отказ {label}")
        if label == "backup-cf":
            Path(arguments[1]).write_bytes(b"original CF")
        elif "/DumpConfigToFiles" in arguments:
            root = Path(arguments[1])
            (root / "Configuration.xml").write_bytes(CONFIG)
            (root / "ConfigDumpInfo.xml").write_text(label)
            if label == self.drift:
                (root / "unexpected.xml").write_text("changed")
        elif label == "run" and self.output is not None:
            (self.work / "io/platform.raw.txt").write_bytes(self.output.encode("utf-8"))


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.work = Path(self.directory.name)
        (self.work / "io").mkdir()
        self.script = self.work / "probe.bsl"
        self.script.write_text('Сообщить("file.begin" + Символ(9) + "probe");\n'
                               'Сообщить("file.end" + Символ(9) + "probe");\n')

    def tearDown(self):
        self.directory.cleanup()

    def run_measure(self, native):
        RUNNER.measure(self.script, "probe", self.work, native)

    def test_success_publishes_only_after_verified_restore(self):
        native = Native(self.work)
        self.run_measure(native)
        self.assertEqual(native.calls, ["backup-cf", "backup-xml", "before-load", "load",
                                       "check", "update", "run", "restore-cf",
                                       "restore-update", "restore-xml"])
        self.assertEqual((self.work / "platform.txt").read_text(), OUTPUT.lstrip("\ufeff").replace("\r\n", "\n"))
        self.assertTrue((self.work / "restoration.txt").is_file())
        self.assertTrue((self.work / "original.cf").is_file())
        config = (self.work / "probe-xml/Configuration.xml").read_text()
        self.assertEqual(config.count("<DataProcessor>Замеры</DataProcessor>"), 1)
        application = (self.work / "probe-xml/Ext/ManagedApplicationModule.bsl").read_text()
        self.assertNotIn("ОбработкаОтображенияОшибки", application)
        for directory in ["original-xml", "probe-xml", "before-load-xml", "restored-xml"]:
            self.assertEqual((self.work / directory).stat().st_mode & 0o777, 0o700)
        self.assertEqual((self.work / "original.cf").stat().st_mode & 0o777, 0o600)

    def test_backup_failure_never_loads_configuration(self):
        for step in ["backup-cf", "backup-xml", "before-load"]:
            with self.subTest(step=step), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                native = Native(work, fail=step)
                with self.assertRaises(RuntimeError):
                    RUNNER.measure(self.script, "probe", work, native)
                self.assertNotIn("load", native.calls)
                self.assertNotIn("restore-cf", native.calls)

    def test_client_error_observer_is_explicit_and_preserves_restore(self):
        native = Native(self.work)
        RUNNER.measure(self.script, "probe", self.work, native, observe_client_errors=True)
        module = (self.work / "probe-xml/Ext/ManagedApplicationModule.bsl").read_text()
        self.assertIn("Процедура ОбработкаОтображенияОшибки(", module)
        self.assertIn("ИнформацияОшибкиПробы, ТребуетсяЗавершениеСеанса, СтандартнаяОбработка)", module)
        self.assertNotIn("ТребуетсяЗавершениеСеанса =", module)
        self.assertIn("СтандартнаяОбработка = Ложь;", module)
        self.assertIn("client.error", module)
        self.assertIn(str(self.work / "io/platform.raw.txt"), module)
        self.assertEqual(native.calls[-3:], ["restore-cf", "restore-update", "restore-xml"])

    def test_empty_cf_is_not_a_backup_even_when_native_succeeds(self):
        with self.assertRaisesRegex(RuntimeError, "резервная копия"):
            self.run_measure(lambda *args: None)
        self.assertFalse((self.work / "probe-xml").exists())

    def test_concurrent_change_is_detected_before_loading(self):
        native = Native(self.work, drift="before-load")
        with self.assertRaisesRegex(RuntimeError, "изменилась"):
            self.run_measure(native)
        self.assertNotIn("load", native.calls)

    def test_every_post_load_failure_restores_including_partial_load(self):
        for step in ["load", "check", "update", "run"]:
            with self.subTest(step=step), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                native = Native(work, fail=step)
                with self.assertRaisesRegex(RuntimeError, f"отказ {step}"):
                    RUNNER.measure(self.script, "probe", work, native)
                self.assertEqual(native.calls[-3:], ["restore-cf", "restore-update", "restore-xml"])
                self.assertTrue((work / "restoration.txt").is_file())
                self.assertFalse((work / "platform.txt").exists())

    def test_missing_empty_or_invalid_output_is_not_success(self):
        cases = [None, "", "file.end\tprobe\nfile.begin\tprobe\n",
                 OUTPUT + "file.end\tprobe\n", OUTPUT + "file.begin\tother\n",
                 OUTPUT + "file.setup_error\tfailure\n", OUTPUT + "file.cleanup_error\tfailure\n"]
        for output in cases:
            with self.subTest(output=output), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                (work / "io").mkdir()
                native = Native(work, output=output)
                with self.assertRaises((RuntimeError, FileNotFoundError)):
                    RUNNER.measure(self.script, "probe", work, native)
                self.assertEqual(native.calls[-1], "restore-xml")
                self.assertTrue((work / "restoration.txt").is_file())
                self.assertFalse((work / "platform.txt").exists())

    def test_restore_failure_keeps_raw_evidence_but_does_not_publish(self):
        for step in ["restore-cf", "restore-update", "restore-xml", "drift"]:
            with self.subTest(step=step), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                (work / "io").mkdir()
                native = Native(work, fail=step, drift="restore-xml" if step == "drift" else None)
                with self.assertRaisesRegex(RuntimeError, "ВОССТАНОВЛЕНИЕ НЕ ПОДТВЕРЖДЕНО"):
                    RUNNER.measure(self.script, "probe", work, native)
                self.assertTrue((work / "original.cf").is_file())
                self.assertTrue((work / "io/platform.raw.txt").is_file())
                self.assertFalse((work / "platform.txt").exists())
                self.assertFalse((work / "restoration.txt").exists())

    def test_interrupt_after_load_still_restores(self):
        native = Native(self.work)
        def interrupt(label, *args):
            if label == "run":
                raise KeyboardInterrupt()
            native(label, *args)
        with self.assertRaises(KeyboardInterrupt):
            self.run_measure(interrupt)
        self.assertEqual(native.calls[-3:], ["restore-cf", "restore-update", "restore-xml"])

    def test_only_root_dump_info_is_excluded(self):
        (self.work / "Configuration.xml").write_bytes(CONFIG)
        (self.work / "ConfigDumpInfo.xml").write_text("ignored")
        nested = self.work / "nested"
        nested.mkdir()
        (nested / "ConfigDumpInfo.xml").write_text("not ignored")
        self.assertIn("nested/ConfigDumpInfo.xml", RUNNER.xml_snapshot(self.work))
        (nested / "link").symlink_to(self.script)
        with self.assertRaisesRegex(RuntimeError, "Ссылка"):
            RUNNER.xml_snapshot(self.work)

    def test_timeout_stops_the_child_group_before_returning(self):
        process = Mock(pid=12345)
        process.wait.side_effect = [subprocess.TimeoutExpired("native", 1), 0, 0]
        with patch.object(RUNNER.subprocess, "Popen", return_value=process) as popen, \
                patch.object(RUNNER.os, "killpg") as kill:
            with self.assertRaises(subprocess.TimeoutExpired):
                RUNNER.run_native("/fake/1cv8", 1, self.work, "run", "ENTERPRISE")
        self.assertEqual(kill.call_args_list, [unittest.mock.call(12345, signal.SIGTERM),
                                              unittest.mock.call(12345, signal.SIGKILL)])
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        self.assertEqual(popen.call_args.args[0][2:4], ["/S", "localhost/test"])

    def test_native_uses_explicit_working_directory_only_when_requested(self):
        process = Mock()
        process.wait.return_value = 0
        cwd = self.work / "enterprise-cwd"
        cwd.mkdir()
        with patch.object(RUNNER.subprocess, "Popen", return_value=process) as popen:
            RUNNER.run_native("/fake/1cv8", 1, self.work, "run", "ENTERPRISE", cwd=cwd)
        self.assertEqual(popen.call_args.kwargs["cwd"], cwd)
        self.assertTrue(popen.call_args.kwargs["start_new_session"])

    def test_acl_does_not_make_backups_readable_to_server(self):
        work = self.work / "acl"
        work.mkdir()
        with patch.object(RUNNER.subprocess, "run") as run:
            RUNNER.prepare_io(work, os.getuid() + 1)
        self.assertEqual(run.call_args_list[0].args[0][2], f"u:{os.getuid() + 1}:x")
        self.assertEqual(run.call_args_list[1].args[0][-1], str(work / "io"))
        self.assertEqual((work / "io").stat().st_mode & 0o777, 0o700)

    def test_cli_requires_explicit_mutation_flag(self):
        result = subprocess.run([sys.executable, str(SCRIPT), str(self.script), "--marker", "probe"],
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Требуется --allow-config-mutation", result.stderr)
        self.assertNotIn("Артефакты:", result.stdout)

    def test_temporary_observer_reads_only_the_four_explicit_files(self):
        with ExitStack() as stack:
            paths = []
            output = ""
            for role in ["closed", "released", "open", "name"]:
                file = stack.enter_context(tempfile.NamedTemporaryFile(prefix="v8_", dir="/tmp"))
                file.write(b"ABC")
                file.flush()
                paths.append(Path(file.name))
                output += f"lifetime.path.{role}\t{file.name}\n"
            RUNNER.observe_temporary_files(output, self.work)
            observed = (self.work / "host-after-enterprise.tsv").read_text().splitlines()
            self.assertEqual(len(observed), 5)
            self.assertTrue(all("\tfile\t" in line and line.endswith("\t3") for line in observed[1:]))
            self.assertTrue(all(path.read_bytes() == b"ABC" for path in paths))
        RUNNER.observe_temporary_files(output, self.work)
        observed = (self.work / "host-after-enterprise.tsv").read_text().splitlines()
        self.assertTrue(all(line.endswith("\tabsent\t-\t-") for line in observed[1:]))

    def test_bad_observer_input_restores_and_does_not_read_foreign_paths(self):
        output = OUTPUT + "lifetime.path.closed\t/etc/passwd\n"
        native = Native(self.work, output=output)
        with patch.object(RUNNER.Path, "lstat") as metadata:
            with self.assertRaisesRegex(RuntimeError, "Некорректный путь"):
                RUNNER.measure(self.script, "probe", self.work, native, observe_temp_lifetime=True)
        metadata.assert_not_called()
        self.assertTrue((self.work / "restoration.txt").is_file())
        self.assertFalse((self.work / "platform.txt").exists())

    def test_observation_precedes_restore(self):
        native = Native(self.work)
        observed = []
        def call(label, *args):
            if label == "restore-cf":
                self.assertEqual(observed, [True])
            native(label, *args)
        with patch.object(RUNNER, "observe_temporary_files", side_effect=lambda *_: observed.append(True)):
            RUNNER.measure(self.script, "probe", self.work, call, observe_temp_lifetime=True)

    def test_real_timeout_kills_a_descendant_that_ignores_term(self):
        executable = self.work / "fake-native"
        executable.write_text("#!" + sys.executable + "\n" + '''
import os, pathlib, signal, subprocess, sys
if sys.argv[1] == "child":
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    print("ready", flush=True)
    while True:
        signal.pause()
child = subprocess.Popen([sys.executable, __file__, "child"], stdout=subprocess.PIPE, text=True)
assert child.stdout.readline().strip() == "ready"
pathlib.Path(sys.argv[sys.argv.index("/Out") + 1]).write_text(str(child.pid))
while True:
    signal.pause()
''')
        executable.chmod(0o700)
        with self.assertRaises(subprocess.TimeoutExpired):
            RUNNER.run_native(str(executable), 2, self.work, "timeout", "ENTERPRISE")
        child_pid = int((self.work / "timeout.log").read_text())
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                state = Path(f"/proc/{child_pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
            except FileNotFoundError:
                break
            if state == "Z":
                break
            time.sleep(0.01)
        else:
            self.fail("Дочерний процесс продолжает исполняться после таймаута")

    def test_real_acl_grants_traversal_but_not_listing_of_private_backups(self):
        work = self.work / "real-acl"
        work.mkdir(mode=0o700)
        server_uid = os.getuid() + 1
        RUNNER.prepare_io(work, server_uid)
        parent = subprocess.check_output(["getfacl", "-ncp", str(work)], text=True)
        child = subprocess.check_output(["getfacl", "-ncp", str(work / "io")], text=True)
        self.assertIn(f"user:{server_uid}:--x\n", parent)
        self.assertIn("other::---\n", parent)
        self.assertIn(f"user:{server_uid}:rwx\n", child)
        self.assertIn(f"default:user:{os.getuid()}:rwx\n", child)
        self.assertIn("other::---\n", child)

    def test_existing_processor_is_not_registered_twice_and_source_is_unchanged(self):
        original = self.work / "source"
        original.mkdir()
        config = CONFIG.replace(b"<ChildObjects/>",
                                "<ChildObjects><DataProcessor>Замеры</DataProcessor></ChildObjects>".encode())
        (original / "Configuration.xml").write_bytes(config)
        (original / "user.xml").write_bytes(b"user metadata")
        before = RUNNER.xml_snapshot(original)
        output = self.work / "prepared"
        RUNNER.prepare_configuration(original, output, self.script, self.work / "io/out.txt")
        self.assertEqual(RUNNER.xml_snapshot(original), before)
        self.assertEqual((output / "Configuration.xml").read_bytes(), config)
        self.assertEqual((output / "user.xml").read_bytes(), b"user metadata")


if __name__ == "__main__":
    unittest.main()
