// Чередующийся A/B для независимых кандидатов этого исследования.
// Основан на скрипте замеров скобок серии разделения модулей.
// node measure.mjs <каталог worktree и target-dir> <кандидат> <новый каталог результатов>
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';

const [root, candidate, output] = process.argv.slice(2);
if (!root || !candidate || !output || candidate === 'base') throw Error('Нужны база, кандидат и новый каталог результатов');
const out = path.resolve(output);
fs.mkdirSync(out);
const scenarios = ['bmp_rotate', 'str_find', 'pi_leibniz', 'call_overhead'];
const sides = ['base', candidate];
const binary = side => path.join(root, side + '-target', 'release/bsl-cli');
const save = (name, value) => fs.writeFileSync(path.join(out, name), JSON.stringify(value, null, 2) + '\n');
const records = [];
function conditions() {
  return {
    time: new Date().toISOString(), load: os.loadavg(),
    ac: fs.readFileSync('/sys/class/power_supply/AC/online', 'utf8').trim(),
    noTurbo: fs.readFileSync('/sys/devices/system/cpu/intel_pstate/no_turbo', 'utf8').trim(),
    governors: fs.readdirSync('/sys/devices/system/cpu').filter(x => /^cpu\d+$/.test(x))
      .map(x => fs.readFileSync('/sys/devices/system/cpu/' + x + '/cpufreq/scaling_governor', 'utf8').trim()),
  };
}
function allowed(c) {
  return c.ac === '1' && c.noTurbo === '1' && c.governors.every(x => x === 'performance') && c.load[0] <= 1.5;
}
async function quiet() {
  for (;;) {
    const c = conditions();
    if (allowed(c)) return c;
    console.log('Ожидание условий: ' + JSON.stringify(c));
    await new Promise(resolve => setTimeout(resolve, 30000));
  }
}
function useful(text) {
  const lines = text.trim().split(/\r?\n/);
  if (lines.length < 2 || !/^\d+$/.test(lines.at(-1))) throw Error('Нет полезного вывода либо итогового времени');
  return lines.slice(0, -1).join('\n');
}
function run(side, scenario, prefix, measured) {
  const args = [binary(side), 'benchmarks/' + scenario + '.bsl'];
  const command = measured ? 'perf' : args.shift();
  const argv = measured ? ['stat', '-x;', '-o', path.join(out, prefix + '.perf'), '-e', 'instructions:u,cycles:u', '--', ...args] : args;
  const result = spawnSync(command, argv, {cwd: path.join(root, side), encoding: 'utf8', env: {...process.env, LC_ALL: 'C'}});
  fs.writeFileSync(path.join(out, prefix + '.out'), result.stdout ?? '');
  fs.writeFileSync(path.join(out, prefix + '.err'), result.stderr ?? '');
  if (result.status !== 0 || result.stderr) throw Error('Ошибка прогона ' + prefix + ': ' + result.status + ' ' + result.stderr);
  return useful(result.stdout);
}
try {
  save('binaries.json', sides.map(side => ({side, path: binary(side), sha256: createHash('sha256').update(fs.readFileSync(binary(side))).digest('hex')})));
  save('status.json', {status: 'running', scenarios, pairs: 7, admission: 'AC=1, performance, no_turbo=1, load1<=1.5'});
  for (const scenario of scenarios) {
    let expected;
    for (const side of sides) {
      await quiet();
      const text = run(side, scenario, scenario + '-' + side + '-warmup', false);
      if (expected !== undefined && text !== expected) throw Error('Различается вывод прогрева');
      expected = text;
    }
    for (let pair = 1; pair <= 7; pair++) {
      for (const side of pair % 2 ? sides : [...sides].reverse()) {
        const before = await quiet();
        const prefix = scenario + '-' + pair + '-' + side;
        const text = run(side, scenario, prefix, true);
        const after = conditions();
        const record = {scenario, pair, side, before, after};
        records.push(record);
        save('records.json', records);
        if (!allowed(after)) throw Error('Условия нарушены во время серии: ' + prefix);
        if (text !== expected) throw Error('Изменился полезный вывод: ' + prefix);
        const raw = fs.readFileSync(path.join(out, prefix + '.perf'), 'utf8');
        for (const event of ['instructions:u', 'cycles:u']) {
          const rows = raw.split('\n').map(x => x.split(';')).filter(x => x[2] === event);
          if (rows.length !== 1 || !/^[1-9]\d*$/.test(rows[0][0]) || Number(rows[0][4]) !== 100) throw Error('Нет полного счётчика ' + event + ': ' + raw);
          record[event] = Number(rows[0][0]);
        }
        save('records.json', records);
        console.log(prefix + ' PASS');
      }
    }
  }
  if (records.length !== 56) throw Error('Неполная серия');
  save('status.json', {status: 'complete', runs: records.length});
} catch (error) {
  save('status.json', {status: 'rejected', reason: String(error), runs: records.length});
  throw error;
}
