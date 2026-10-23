"""Снимает одновременно удерживаемые номера двух сеансов временной базы 1С."""

import os, signal, subprocess, time
from pathlib import Path
root=Path('/tmp/open-bsl-number-concurrent-096')
root.mkdir(exist_ok=True)
for name in ['first.ready','second.ready']:
    (root/name).unlink(missing_ok=True)
env=os.environ.copy()
env.update(ONEC_PLATFORM='/opt/1cv8/x86_64/8.3.27.2342/1cv8', ONEC_ENTERPRISE_PLATFORM='/tmp/open-bsl-onec-ru-fixed', ONEC_IB='/tmp/open-bsl-number-concurrent-096-ib', ONEC_TIMEOUT='90')
first_log=(root/'first.log').open('w')
second_log=(root/'second.log').open('w')
first=subprocess.Popen(['./tests/conformance/measure/1c/run-on-1c.sh','tests/conformance/measure/1c/measure-document-new-number-concurrent.bsl'],env=env,stdout=first_log,stderr=subprocess.STDOUT,start_new_session=True)
second=None
try:
    deadline=time.monotonic()+90
    while not (root/'first.ready').exists() or not (root/'first.ready').stat().st_size:
        if first.poll() is not None: raise RuntimeError('first run terminated before reserving')
        if time.monotonic()>deadline: raise RuntimeError('first reservation timeout')
        time.sleep(.05)
    if first.poll() is not None: raise RuntimeError('first session not live before second launch')
    native_env=env.copy()
    native_env.pop('WAYLAND_DISPLAY',None)
    native_env.update(GDK_BACKEND='x11', DISPLAY=env.get('DISPLAY',':1'), XDG_RUNTIME_DIR=env.get('XDG_RUNTIME_DIR',f'/run/user/{os.getuid()}'),LD_LIBRARY_PATH=str(Path.home()/'.local/lib/1c-wk41')+':/usr/lib')
    second=subprocess.Popen(['/tmp/open-bsl-onec-ru-fixed','ENTERPRISE','/F'+env['ONEC_IB'],'/DisableStartupDialogs','/DisableStartupMessages','/Out',str(root/'second-native.log')],env=native_env,stdout=second_log,stderr=subprocess.STDOUT,start_new_session=True)
    if first.wait(timeout=90)!=0: raise RuntimeError('first run failed; inspect first.log')
    if second.wait(timeout=30)!=0: raise RuntimeError('second native session failed')
    out=Path('tests/conformance/measure/1c/measure-document-new-number-concurrent.platform.txt')
    native_output=out.read_text(encoding='utf-8-sig').strip()
    if 'document.number.concurrent.error=' in native_output: raise RuntimeError('native reservation probe failed')
    rows=[]
    for name in ['first.ready','second.ready']:
        raw=(root/name).read_bytes()
        text=raw.decode('utf-16' if raw.startswith((b'\xff\xfe',b'\xfe\xff')) else 'utf-8-sig').strip()
        if len(text.splitlines())!=1: raise RuntimeError('unexpected ready artifact')
        rows.append(text)
    if native_output != rows[0]: raise RuntimeError('first object number changed or output is incomplete')
    out=Path('tests/conformance/measure/1c/measure-document-new-number-concurrent.platform.txt')
    out.write_text('\n'.join(rows)+'\n')
    print('\n'.join(rows))
finally:
    for process in [second,first]:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: os.killpg(process.pid, signal.SIGKILL); process.wait()
    first_log.close(); second_log.close()
