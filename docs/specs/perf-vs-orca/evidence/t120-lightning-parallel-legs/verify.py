#!/usr/bin/env python3
"""Verify the reduced e2e measurement and its roster, not the runtime itself."""
from collections import Counter
from copy import deepcopy
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
TEST = re.compile(r'^test (\S+) \.\.\. (ok|FAILED|ignored)(?:,.*)?$')
SUMMARY = re.compile(
    r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
    r'(\d+) measured; (\d+) filtered out; finished in ([\d.]+)s$'
)


def parse(log):
    rows = []
    summaries = []
    for line in log.splitlines():
        if match := TEST.match(line):
            rows.append((match[1], match[2]))
        if match := SUMMARY.match(line):
            summaries.append(match.groups())
    if len(summaries) != 1:
        raise ValueError('expected exactly one libtest summary')
    state, passed, failed, ignored, measured, filtered, seconds = summaries[0]
    return rows, (state, int(passed), int(failed), int(ignored), int(measured),
                  int(filtered), float(seconds))


def check(data, before_log, after_log, reference):
    problems = []
    parsed = []
    for label, log in [('before', before_log), ('after', after_log)]:
        try:
            rows, summary = parse(log)
        except ValueError as error:
            problems.append(f'{label}: {error}')
            continue
        parsed.append((rows, summary))
        names = [name for name, _ in rows]
        counts = Counter(status for _, status in rows)
        if len(names) != len(set(names)):
            problems.append(f'{label}: duplicate test result')
        if summary[:6] != ('ok', 172, 0, 1, 0, 0):
            problems.append(f'{label}: unexpected libtest summary {summary}')
        if counts != Counter({'ok': 172, 'ignored': 1}):
            problems.append(f'{label}: incomplete or failed test roster {counts}')
        if summary[6] != data[label]['bucket_seconds']:
            problems.append(f'{label}: recorded time differs from libtest')
    if len(parsed) == 2 and dict(parsed[0][0]) != dict(parsed[1][0]):
        problems.append('before and after test rosters or outcomes differ')
    if abs(data['difference_seconds'] -
           (data['after']['bucket_seconds'] - data['before']['bucket_seconds'])) > 0.001:
        problems.append('recorded time difference is wrong')
    if data['calicat_before_sha256'] != reference or data['calicat_after_sha256'] != reference:
        problems.append('calicat hash differs from the recorded reference')
    return problems


def main():
    data = json.loads((HERE / 'summary.json').read_text(encoding='utf-8'))
    before = (HERE / 'before.log').read_text(encoding='utf-8')
    after = (HERE / 'after.log').read_text(encoding='utf-8')
    reference = json.loads((HERE.parent / 't118-component-cache' / 'summary.json')
                           .read_text(encoding='utf-8'))['slice']['gcode_sha256']
    errors = check(data, before, after, reference)
    if errors:
        for error in errors:
            print(f'FAIL: {error}')
        return 1
    print('PASS: results, roster, timing arithmetic and reference hash agree')
    if '--self-test' in sys.argv:
        bad_hash = deepcopy(data)
        bad_hash['calicat_after_sha256'] = '0' * 64
        bad_time = deepcopy(data)
        bad_time['difference_seconds'] += 1
        bad_summary = deepcopy(data)
        bad_summary['after']['bucket_seconds'] += 1
        first_result = next(line for line in after.splitlines() if TEST.match(line))
        controls = [
            ('missing test', data, before, after.replace(first_result + '\n', '', 1)),
            ('changed outcome', data, before, after.replace(first_result, first_result[:-2] + 'FAILED', 1)),
            ('duplicate test', data, before, after + '\n' + first_result),
            ('altered reported time', bad_summary, before, after),
            ('incorrect difference', bad_time, before, after),
            ('incorrect hash', bad_hash, before, after),
        ]
        for name, control_data, control_before, control_after in controls:
            if not check(control_data, control_before, control_after, reference):
                print(f'MISSED: {name}')
                return 1
            print(f'rejected: {name}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
