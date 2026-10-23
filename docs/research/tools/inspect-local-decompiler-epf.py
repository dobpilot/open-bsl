"""Статически извлекает артефакты обработки; EPF и DLL не исполняются."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('epf', type=Path)
parser.add_argument('vendor', type=Path)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
sys.path.insert(0, str(args.vendor / 'src'))
from v8unpack.container import Container
from v8unpack import helper

args.output.mkdir(parents=True, exist_ok=False)
with tempfile.TemporaryDirectory(prefix='open-bsl-local-decompiler-') as temporary:
    with args.epf.open('rb') as stream:
        container = Container()
        # Первый указатель может адресовать свободный блок, а не ff ff ff 7f.
        container.read(stream)
        container.extract(temporary, deflate=True, recursive=True)
    images = list(Path(temporary).rglob('image'))
    assert len(images) == 1
    image_path = images[0]
    image = image_path.read_bytes()
    parsed = helper.brace_file_read(str(image_path.parent), 'image')[0]
    blocks = {block[0].strip('"'): block for block in parsed[1:]}
    commands = [(int(code), int(operand)) for code, operand in blocks['Cmd'][3:]]
    # Эти точные участки подтверждают преобразование строки конкретной версии.
    assert commands[2940:2949] == [(2, 281), (4, 25), (16, 0), (2, 332),
                                   (4, 5), (53, 0), (16, 0), (2, 22), (4, 21)]
    assert commands[12133:12138] == [(2, 22), (2, 22), (4, 5), (2, 281), (42, 7)]
    assert commands[11243:11248] == [(64, 0), (2, 281), (25, 0), (4, 18), (39, 3241)]
    assert commands[3241:3245] == [(25, 0), (63, 0), (24, 0), (16, 0)]
    assert commands[8185:8187] == [(2, 22), (52, 0)]
    assert blocks['Const'][20] == ['"N"', '288']
    quoted = blocks['Const'][7][1]
    assert quoted.startswith('"') and quoted.endswith('"')
    encoded = quoted[1:-1].replace('""', '"')
    decoded = ''.join(chr(ord(ch) - position - 288)
                      for position, ch in enumerate(encoded, start=1))
    (args.output / 'initialization.bsl').write_text(decoded)
    match = re.search(r'(?ms)^фt="(.*?)";', decoded)
    assert match
    rows = [line.lstrip('|').split('\t') for line in match[1].splitlines()
            if line.lstrip('|').split('\t')[0].isdigit()]
    assert [int(row[0]) for row in rows] == list(range(152))
    with (args.output / 'opcode-table.csv').open('w', newline='') as file:
        writer = csv.writer(file)
        writer.writerow(['opcode', 'pop', 'push', 'text', 'generator_kind',
                         'precedence', 'special_kind'])
        writer.writerows(rows)
    summary = {
        'epf_sha256': hashlib.sha256(args.epf.read_bytes()).hexdigest(),
        'image_sha256': hashlib.sha256(image).hexdigest(),
        'image_bytes': len(image), 'commands': len(commands),
        'module_entry': int(blocks['Cmd'][2]),
        'constants': int(blocks['Const'][1]), 'globals': int(blocks['Var'][1]),
        'procedures': int(blocks['Proc'][1]), 'opcode_table_rows': len(rows),
        'special_kind_99': [int(row[0]) for row in rows if row[-1] == '99'],
        'decoded_initialization_characters': len(decoded),
        'executed_epf_or_dll': False,
    }
    (args.output / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(summary, ensure_ascii=False, indent=2))
