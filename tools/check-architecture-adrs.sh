#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
python3 -I - "$root" <<'CHECK'
from pathlib import Path
import re,sys
root=Path(sys.argv[1])
index=(root/'docs/ADRs/core.org').read_text()
actual=set()
for p in sorted((root/'docs/ADRs').glob('[0-9][0-9][0-9][0-9]-*.org')):
    id='ADR-'+p.name[:4]; actual.add(id)
    s=p.read_text()
    status=re.search(r'^#\+status: (.*)$',s,re.M)
    assert status and status[1] in {'Accepted','Superseded'}, f'invalid decision status: {p}'
    if status[1]=='Superseded':
        assert '0025-semantic-engine-and-binding-runtimes.org' in s, f'missing successor: {p}'
    assert id in index and p.name in index, f'unindexed decision: {p}'
assert actual==set(re.findall(r'ADR-[0-9]{4}',index)), 'decision index differs'
for path in ['docs/design.md','ARCHITECTURE_GOVERNANCE.md','docs/spec/authority.toml']:
    assert '0025' in (root/path).read_text(), f'missing successor decision: {path}'
assert '`MOD-DEPS-001`:' in (root/'docs/architecture/20-module-boundaries.md').read_text()
print('architecture ADR check: current and superseded decision scopes valid')
CHECK
