"""Извлекает справочные сведения v8asm; они не заменяют измерения на 1С."""
import argparse
import csv
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('vendor', type=Path)
args = parser.parse_args()
path = args.vendor / 'src/ExternalDataProcessors/Ассемблер/Templates/Описание/Template.mxlx'
namespaces = {'s': 'http://v8.1c.ru/8.2/data/spreadsheet',
              'v': 'http://v8.1c.ru/8.1/data/core'}
rows = []
for item in ET.parse(path).getroot().findall('s:rowsItem', namespaces):
    cells = ['\n'.join(text.text or '' for text in cell.findall('.//v:content', namespaces))
             for cell in item.findall('s:row/s:c', namespaces)]
    if len(cells) > 1 and cells[1].isdigit():
        # Сохраняем мнемонику и численные эффекты, без догадок для пустых полей.
        rows.append((int(cells[1]), cells[0], cells[4] if len(cells) > 4 else '',
                     cells[5] if len(cells) > 5 else '',
                     cells[6] if len(cells) > 6 else ''))
assert [row[0] for row in rows] == list(range(129))
writer = csv.writer(sys.stdout)
writer.writerow(['opcode', 'v8asm_mnemonic', 'stack_pop', 'stack_push', 'bsl_name'])
writer.writerows(rows)
