// Сводка полной принятой серии; сценарии не усредняются между собой.
// node summarize.mjs <каталог результатов measure.mjs>
import fs from 'node:fs';
import path from 'node:path';

const directory = process.argv[2];
const read = name => JSON.parse(fs.readFileSync(path.join(directory, name), 'utf8'));
if (read('status.json').status !== 'complete') throw Error('Серия не завершена либо отвергнута');
const records = read('records.json');
if (records.length !== 56) throw Error('Нужны четыре сценария по семь пар');
function stats(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return {median: sorted[3], min: sorted[0], max: sorted[6]};
}
const scenarios = ['bmp_rotate', 'str_find', 'pi_leibniz', 'call_overhead'];
for (const scenario of scenarios) {
  const rows = records.filter(row => row.scenario === scenario);
  const candidates = [...new Set(rows.filter(row => row.side !== 'base').map(row => row.side))];
  if (rows.length !== 14 || candidates.length !== 1) throw Error('Неполный либо смешанный сценарий: ' + scenario);
  const result = {scenario, candidate: candidates[0]};
  for (const event of ['instructions:u', 'cycles:u']) {
    const base = [], candidate = [], deltas = [];
    for (let pair = 1; pair <= 7; pair++) {
      const ab = rows.filter(row => row.pair === pair);
      const a = ab.find(row => row.side === 'base');
      const b = ab.find(row => row.side === candidates[0]);
      if (ab.length !== 2 || !a || !b) throw Error('Неполная пара');
      if (![a[event], b[event]].every(n => Number.isSafeInteger(n) && n > 0)) throw Error('Неверный счётчик');
      base.push(a[event]);
      candidate.push(b[event]);
      deltas.push((b[event] / a[event] - 1) * 100);
    }
    result[event] = {base: stats(base), candidate: stats(candidate), pairedPercent: {...stats(deltas), deltas}};
  }
  result.loadBefore = [Math.min(...rows.map(row => row.before.load[0])), Math.max(...rows.map(row => row.before.load[0]))];
  result.loadAfter = [Math.min(...rows.map(row => row.after.load[0])), Math.max(...rows.map(row => row.after.load[0]))];
  console.log(JSON.stringify(result));
}
