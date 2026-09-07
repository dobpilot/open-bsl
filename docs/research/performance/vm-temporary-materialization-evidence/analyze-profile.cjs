// Разбор исходного профиля, параметризованный для независимых сторон A/B.
// Оценочные счётчики функций — доли отсчётов, умноженные на медиану процесса;
// это не точные счётчики функций и не цена инструкции с обращением к rsp.
const fs = require('fs');
const cp = require('child_process');
const [root, binary] = process.argv.slice(2);
if (!root || !binary) throw Error('Нужны каталог данных и бинарник соответствующей стороны');
const names = {step: '_RNvCsbxgXiyEKxkX_6bsl_vm4step', step_cold: '_RNvCsbxgXiyEKxkX_6bsl_vm9step_cold'};
const code = {};
for (const [name, symbol] of Object.entries(names)) {
    const dump = cp.execFileSync('objdump', ['-d', '-Mintel', '--no-show-raw-insn', '--disassemble=' + symbol, binary], {encoding: 'utf8'});
    const base = parseInt(dump.match(new RegExp('([0-9a-f]+) <' + symbol + '>:'))[1], 16);
    code[name] = new Map([...dump.matchAll(/^\s*([0-9a-f]+):\s+(.+)$/gm)].map(m => [parseInt(m[1], 16) - base, m[2]]));
}
const median = a => [...a].sort((a,b) => a-b)[Math.floor(a.length / 2)];
const range = a => ({median: median(a), min: Math.min(...a), max: Math.max(...a)});
const result = {};
for (const scenario of ['bmp_rotate', 'str_find', 'pi_leibniz', 'call_overhead']) {
    const dir = root + '/' + scenario;
    const expected = fs.readFileSync(dir + '/warmup.stdout', 'utf8').trim().split('\n').slice(0, -1).join('\n');
    const counts = {instructions: [], cycles: [], scriptMs: []};
    for (let i = 1; i <= 7; i++) {
        const stat = fs.readFileSync(dir + '/stat-' + i + '.txt', 'utf8');
        for (const event of ['instructions', 'cycles']) {
            const row = stat.split('\n').find(l => l.includes(';' + event + ':u;')).split(';');
            if (!/^\d+$/.test(row[0]) || Number(row[4]) !== 100) throw Error('invalid counter ' + scenario);
            counts[event].push(Number(row[0]));
        }
        for (const kind of ['stat', 'profile']) {
            const lines = fs.readFileSync(dir + (kind === 'profile' ? '/profile-v2' : '') + '/' + kind + '-' + i + '.stdout', 'utf8').trim().split('\n');
            const ms = Number(lines.pop());
            if (!Number.isFinite(ms) || lines.join('\n') !== expected) throw Error('output mismatch ' + scenario);
            if (kind === 'stat') counts.scriptMs.push(ms);
        }
    }
    const samples = cp.execFileSync('perf', ['script', '-i', dir + '/profile-v2/perf.data', '-F', 'comm,event,period,ip,sym,symoff,dso'], {encoding: 'utf8', maxBuffer: 256 * 1024 * 1024});
    const events = {};
    for (const line of samples.split('\n')) {
        const m = line.match(/^\s*(\S+)\s+(\d+)\s+(\S+):\s+([0-9a-f]+)\s+(.+)\s+\((.+)\)$/);
        if (!m) { if (line.trim()) throw Error('sample parse: ' + line); continue; }
        if (m[1] !== 'bsl-cli') continue;
        const [, , period, event, , fullSymbol] = m;
        const symbol = fullSymbol.replace(/\+0x[0-9a-f]+$/, '') + (fullSymbol === '[unknown]' ? ' in ' + m[6] : '');
        const e = events[event] ||= {total: 0, samples: 0, symbols: {}, vm: {}};
        e.total += Number(period); e.samples++;
        e.symbols[symbol] = (e.symbols[symbol] || 0) + Number(period);
        for (const name of ['step', 'step_cold']) {
            if (symbol !== 'bsl_vm::' + name) continue;
            const offset = parseInt(fullSymbol.match(/\+0x([0-9a-f]+)$/)?.[1] || '0', 16);
            const v = e.vm[name] ||= {total: 0, offsets: {}};
            v.total += Number(period);
            v.offsets[offset] = (v.offsets[offset] || 0) + Number(period);
        }
    }
    for (const [event, e] of Object.entries(events)) {
        e.top = Object.entries(e.symbols).sort((a,b) => b[1]-a[1]).slice(0,8).map(([symbol, n]) => ({symbol, percent: 100*n/e.total}));
        delete e.symbols;
        for (const [name, v] of Object.entries(e.vm)) {
            v.percent = 100*v.total/e.total;
            v.estimatedCounter = v.total/e.total * median(counts[event.split(':')[0]]);
            v.rspMemoryPercentOfFunction = 100 * Object.entries(v.offsets).filter(([o]) => {const insn=code[name].get(Number(o))||'';return insn.includes('[rsp') && !insn.startsWith('lea');}).reduce((s,[,n])=>s+n,0)/v.total;
            v.top = Object.entries(v.offsets).sort((a,b)=>b[1]-a[1]).slice(0,12).map(([o,n])=>({offset: '0x'+Number(o).toString(16), instruction: code[name].get(Number(o)), percentOfFunction: 100*n/v.total}));
            delete v.offsets;
        }
    }
    result[scenario] = {counts: Object.fromEntries(Object.entries(counts).map(([k,v])=>[k,range(v)])), output: expected, events};
}
console.log(JSON.stringify(result, null, 2));
