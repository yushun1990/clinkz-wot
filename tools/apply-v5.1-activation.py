#!/usr/bin/env python3
from pathlib import Path
import csv
import io

ROOT = Path('.')
REVIEWED = '3b133ebfe3c870102931982d6c056595f9d44255'


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding='utf-8')


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding='utf-8')


def replace_once(path: str, old: str, new: str) -> None:
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{path}: expected one occurrence of {old!r}, found {count}')
    write(path, text.replace(old, new, 1))


# Active authority selector and independent-review provenance.
path = 'docs/spec/v5-authority-reset.toml'
text = read(path)
for old, new in [
    ('current_design_revision = "5.0"', 'current_design_revision = "5.1"'),
    ('target_design_revision = "5.1"\n', ''),
    ('status = "candidate"', 'status = "active"'),
    ('decision = "docs/ADRs/0018-bounded-v5-normative-authority-reset.org"',
     'decision = "docs/ADRs/0019-consumer-one-shot-authority-entry.org"'),
    ('workspace_basis = "workspace/0015-normative-authority-reset.md"',
     f'workspace_basis = "workspace/0061-consumer-one-shot-domain-entry.md"\nreviewed_candidate_commit = "{REVIEWED}"'),
    ('candidate_decision = "docs/ADRs/0019-consumer-one-shot-authority-entry.org"\n', ''),
    ('candidate_workspace_basis = "workspace/0061-consumer-one-shot-domain-entry.md"\n', ''),
]:
    if text.count(old) != 1:
        raise SystemExit(f'{path}: activation anchor mismatch for {old!r}')
    text = text.replace(old, new, 1)
write(path, text)

# Accept ADR-0019 without changing its reviewed semantic decision.
path = 'docs/ADRs/0019-consumer-one-shot-authority-entry.org'
text = read(path)
if text.count('#+status: Proposed') != 1:
    raise SystemExit('ADR-0019 proposed status anchor mismatch')
text = text.replace('#+status: Proposed', '#+status: Accepted', 1)
text = text.replace(
    'The classified requirement total remains 121. The v5.1 candidate contains 65\nactive requirements and 31 =inactive-domain-entry-review-required=\nrequirements.',
    'The classified requirement total remains 121. The active v5.1 authority contains\n65 active requirements and 31 =inactive-domain-entry-review-required=\nrequirements.',
    1,
)
old_activation = '''This ADR is part of a docs-only v5.1 authority candidate. The candidate does
not itself admit implementation or create the Consumer Property Read
architecture gate.

The candidate must receive independent architecture-level acceptance at one
immutable head. A separate activation checkpoint then selects v5.1 without
changing the reviewed normative boundary. Only after activation may the
repository register =CONSUMER-PROPERTY-READ-ARCHITECTURE= and admit the
corresponding ADR-0013 source tranches.'''
new_activation = f'''The docs-only v5.1 authority candidate received independent architecture-level
acceptance at exact commit ={REVIEWED}=. The prior ownership blocker for
=validate_untrusted_binding_output= was corrected before that acceptance, and
the reviewed candidate retained exactly 65 active and 31 domain-entry-deferred
identities.

This activation checkpoint selects v5.1 without changing that reviewed
normative boundary. Activation itself creates no Consumer gate manifest and
admits no Rust source tranche. After the activation checkpoint is integrated,
the repository may register =CONSUMER-PROPERTY-READ-ARCHITECTURE= and admit its
exact ADR-0013 source tranches through independent tranche review.'''
if text.count(old_activation) != 1:
    raise SystemExit('ADR-0019 activation paragraph anchor mismatch')
text = text.replace(old_activation, new_activation, 1)
write(path, text)

# Accepted ADR index and review provenance.
path = 'docs/ADRs/core.org'
text = read(path)
anchor = '- [[file:0018-bounded-v5-normative-authority-reset.org][ADR-0018: Bounded v5 normative authority reset]]'
addition = anchor + '\n- [[file:0019-consumer-one-shot-authority-entry.org][ADR-0019: Consumer One-Shot Authority Entry]]'
if text.count(anchor) != 1 or 'ADR-0019: Consumer One-Shot Authority Entry' in text:
    raise SystemExit('ADR index activation anchor mismatch')
text = text.replace(anchor, addition, 1)
text += f'''\nThe v5.1 Consumer one-shot authority entry, independently accepted at candidate
commit ={REVIEWED}= and activated without widening its reviewed requirement
set, is recorded in [[file:0019-consumer-one-shot-authority-entry.org][ADR-0019]].\n'''
write(path, text)

# Source manifest status/count projection.
path = 'docs/design.md'
text = read(path)
text = text.replace('# clinkz-wot v5.0 Authority Manifest', '# clinkz-wot v5.1 Authority Manifest', 1)
text = text.replace(
    'Status: active v5.0 authority, independently reviewed and integrated through\nthe exact ADR-0018 activation checkpoint.',
    f'Status: active v5.1 authority, independently reviewed at candidate commit\n`{REVIEWED}` and integrated through the ADR-0019 activation checkpoint.',
    1,
)
text = text.replace(
    '; ADR-0018 and\n`docs/spec/v5-authority-reset.toml` give every one of its 121 requirement\nidentities an exact disposition.',
    '; ADR-0018 established the bounded reset, ADR-0019 activates the\nConsumer one-shot entry, and `docs/spec/v5-authority-reset.toml` gives every\none of its 121 requirement identities an exact disposition.',
    1,
)
text = text.replace('The 34 `inactive-domain-entry-review-required` identities',
                    'The 31 `inactive-domain-entry-review-required` identities', 1)
text = text.replace('own the 62 active requirements;', 'own the 65 active requirements;', 1)
if 'active v5.0 authority' in text or '62 active requirements' in text or 'The 34 `inactive-domain-entry' in text:
    raise SystemExit('docs/design.md retains stale v5.0 authority projection')
write(path, text)

# Architecture backbone status and proof sequencing projection.
path = 'docs/architecture/README.md'
text = read(path)
text = text.replace('Status: active v5.0 authority.', 'Status: active v5.1 authority.', 1)
text = text.replace('own the 62 active requirements.', 'own the 65 active requirements.', 1)
text = text.replace('ten owners of the 62 active\ndefinitions.', 'ten owners of the 65 active\ndefinitions.', 1)
text = text.replace(
    "ADR-0018 supersedes residual decomposition and ADR-0014's D3 target DAG.",
    "ADR-0018 supersedes residual decomposition and ADR-0014's D3 target DAG.\nADR-0019 activates the reviewed v5.1 Consumer one-shot authority entry.",
    1,
)
text = text.replace(
    'This candidate grants no authority until independent review and separate\nmainline integration. A bounded implementation tranche may proceed only through\nADR-0013 admission; the reset itself creates no source-edit permission.',
    'The active v5.1 authority still grants no source-edit permission by itself. A\nbounded implementation tranche may proceed only through ADR-0013 admission;\nactivation of the Consumer one-shot domain is not implementation admission.',
    1,
)
text = text.replace(
    '1. after an explicit Consumer one-shot domain-entry authority review, a narrow\n   Consumer Property Read proof covering admitted consumed-plan publication,',
    '1. with the Consumer one-shot domain now active in v5.1, a narrow Consumer\n   Property Read proof covering admitted consumed-plan publication,',
    1,
)
write(path, text)

for arch in (ROOT / 'docs/architecture').glob('*.md'):
    arch_text = arch.read_text(encoding='utf-8')
    arch_text = arch_text.replace('Status: active v5.0 authority.', 'Status: active v5.1 authority.')
    arch.write_text(arch_text, encoding='utf-8')

# Active specification status; requirement wording is otherwise unchanged.
replace_once(
    'docs/spec/interaction-core.md',
    'Status: v5.1 activation candidate. v5.0 remains active until the separately\nreviewed activation checkpoint selects this revision.\n\nThis specification owns eleven v5.1 candidate requirements:',
    'Status: active v5.1 authority.\n\nThis specification owns eleven v5.1 active requirements:',
)
for spec_path, old, new in [
    ('docs/spec/planning.md',
     'Status: v5.1 activation candidate. Nine requirement definitions are registered\nfor the candidate; v5.0 remains active until the separate activation checkpoint.',
     'Status: active v5.1 authority. Nine requirement definitions are registered.'),
    ('docs/spec/binding-spi.md',
     'Status: v5.1 activation candidate. Twelve requirement definitions are\nregistered for the candidate; v5.0 remains active until the separate activation\ncheckpoint.',
     'Status: active v5.1 authority. Twelve requirement definitions are registered.'),
]:
    spec_text = read(spec_path)
    if spec_text.count(old) != 1:
        raise SystemExit(f'{spec_path}: active status anchor mismatch')
    spec_text = spec_text.replace(old, new, 1)
    spec_text = spec_text.replace('v5.1 Consumer Property Read candidate', 'v5.1 Consumer Property Read authority')
    spec_text = spec_text.replace('v5.1 candidate', 'v5.1 active authority')
    write(spec_path, spec_text)

# Specification index becomes the active revision record.
path = 'docs/spec/README.md'
text = read(path)
prefix_old = '''Status: v5.1 Consumer one-shot activation candidate. v5.0 remains the active
mainline authority until the separately reviewed activation checkpoint selects
v5.1.

ADR-0018 established the bounded v5.0 authority reset. ADR-0019 is the proposed
v5.1 domain-entry decision that re-adopts exactly three previously deferred
Consumer one-shot identities. `v5-authority-reset.toml` now projects the v5.1
candidate while retaining `current_design_revision = "5.0"` and
`target_design_revision = "5.1"` so review cannot be mistaken for activation.'''
prefix_new = f'''Status: active v5.1 Consumer one-shot authority.

ADR-0018 established the bounded v5.0 reset. ADR-0019 activates the independently
reviewed v5.1 Consumer one-shot entry at candidate commit `{REVIEWED}`. The
active `v5-authority-reset.toml` now selects `current_design_revision = "5.1"`
with 65 active requirements and records that immutable reviewed candidate.'''
if text.count(prefix_old) != 1:
    raise SystemExit('spec README candidate prefix mismatch')
text = text.replace(prefix_old, prefix_new, 1)
text = text.replace('## v5.1 candidate owners', '## v5.1 active owners', 1)
text = text.replace('| Owner | Candidate responsibility | Count |', '| Owner | Active responsibility | Count |', 1)
text = text.replace(
    'Candidate total: 65. The machine manifest is authoritative for the exact\ncandidate identities; this table is navigation. Until activation, the last\nintegrated v5.0 checkpoint remains the current authority for source admission.',
    'Active total: 65. The machine manifest is authoritative for the exact active\nidentities; this table is navigation. Source admission still requires an exact\nADR-0013 tranche; revision activation alone authorizes no Rust edit.',
    1,
)
text = text.replace('- One candidate-active requirement has exactly one registered candidate\n  definition.',
                    '- One active requirement has exactly one registered active definition.', 1)
text = text.replace('  candidate ownership come from `v5-authority-reset.toml`.',
                    '  active ownership comes from `v5-authority-reset.toml`.', 1)
text = text.replace('- A candidate does not admit source implementation merely because its proposed\n  normative text and checker pass.',
                    '- An active revision does not admit source implementation merely because its\n  normative text and checker pass.', 1)
text = text.replace('The candidate moves exactly these identities from\n`inactive-domain-entry-review-required` to candidate-active authority:',
                    'v5.1 moves exactly these identities from\n`inactive-domain-entry-review-required` to active authority:', 1)
text = text.replace('After v5.1 activation, 31 domain-entry identities remain deferred.',
                    'In active v5.1, 31 domain-entry identities remain deferred.', 1)
marker = '## Candidate and activation protocol\n'
if text.count(marker) != 1:
    raise SystemExit('spec README activation section marker mismatch')
text = text.split(marker, 1)[0].rstrip() + f'''\n\n## v5.1 activation record\n\nThe docs-only candidate at `{REVIEWED}` passed independent architecture-level\nacceptance after the Consumer response validator ownership projection was moved\nfrom Planning to Core and enforced by the candidate checker.\n\nThe activation checkpoint:\n\n- accepts and indexes ADR-0019;\n- selects v5.1 as the active authority while preserving 65/31/121;\n- promotes the reviewed Consumer one-shot ownership and metadata checks into the\n  normal active-v5.1 checker path;\n- migrates the reviewed WP-100/WP-200/WP-300/WP-400 Consumer Property Read\n  slices into the active work-package documents;\n- marks workspace/0061 `MIGRATED`; and\n- removes candidate-only checker/package-projection files.\n\nIt does not register `CONSUMER-PROPERTY-READ-ARCHITECTURE`, admit a source\ntranche, or activate any additional deferred identity. Those steps begin only\nafter this activation checkpoint is integrated.\n'''
write(path, text)

# Durable roadmap state reflects completed domain entry, not source completion.
path = 'PLAN.md'
text = read(path)
text = text.replace('Active design revision: v5.0 bounded-core authority',
                    'Active design revision: v5.1 Consumer one-shot authority', 1)
old_frontier = "The v5.0 authority is active. All six narrow Property Read tranches and D48's"
new_frontier = "The v5.1 Consumer one-shot authority is active. The domain-entry review is\ncomplete; no Consumer source tranche or architecture gate is yet admitted. All six\nnarrow Producer Property Read tranches and D48's"
if text.count(old_frontier) != 1:
    raise SystemExit('PLAN frontier activation anchor mismatch')
text = text.replace(old_frontier, new_frontier, 1)
old_critical = '''1. In parallel, execute the existing WP-400 early multi-owner/scheduler
   checkpoint and complete the Consumer one-shot domain-entry authority review.
2. Complete a narrow Consumer Property Read cross-package architecture gate,'''
new_critical = '''1. In parallel, execute the existing WP-400 early multi-owner/scheduler
   checkpoint and admit/complete the exact WP-100 -> WP-200 -> WP-300 -> WP-400
   Consumer Property Read slices under ADR-0013.
2. Complete the narrow Consumer Property Read cross-package architecture gate,'''
if text.count(old_critical) != 1:
    raise SystemExit('PLAN critical-path activation anchor mismatch')
text = text.replace(old_critical, new_critical, 1)
write(path, text)

# Migrate reviewed candidate package slices into active WP-100..400 owners.
projection_path = ROOT / 'docs/work-packages/CONSUMER-PROPERTY-READ-V5.1-CANDIDATE.md'
projection = projection_path.read_text(encoding='utf-8')
sections = [
    ('## WP-100 Consumer call values and response validator', 'docs/work-packages/WP-100-core.md'),
    ('## WP-200 Consumer Property Read planning and selection', 'docs/work-packages/WP-200-planning.md'),
    ('## WP-300 selected OutboundRequest and ClientBinding call', 'docs/work-packages/WP-300-bindings.md'),
    ('## WP-400 consumed plan-set and call ownership', 'docs/work-packages/WP-400-servient.md'),
]
for header, target in sections:
    start = projection.find(header)
    if start < 0:
        raise SystemExit(f'missing candidate package section {header}')
    next_header = projection.find('\n## ', start + len(header))
    if next_header < 0:
        next_header = len(projection)
    section = projection[start:next_header].strip()
    section = section.replace(header, '## v5.1 Consumer Property Read entry slice', 1)
    section = section.replace('Candidate requirements:', 'Active authority consumed by this slice:', 1)
    target_text = read(target)
    marker = '## v5.1 Consumer Property Read entry slice'
    if marker in target_text:
        raise SystemExit(f'{target}: slice already migrated')
    anchor = '\n## Requirements\n'
    if target_text.count(anchor) != 1:
        raise SystemExit(f'{target}: Requirements anchor is not unique')
    target_text = target_text.replace(anchor, '\n\n' + section + '\n\n## Requirements\n', 1)
    write(target, target_text)
projection_path.unlink()

replace_once('docs/work-packages/index.toml', 'design_revision = "5.0"', 'design_revision = "5.1"')

# Promote current artifact-registry rows touched by activation; preserve historical v5.0 proof artifacts.
registry_path = ROOT / 'docs/artifacts.csv'
rows = list(csv.reader(io.StringIO(registry_path.read_text(encoding='utf-8'))))
expected_header = ['path','role','normativity','design_revision','schema_version','requirement_source']
if rows[0] != expected_header:
    raise SystemExit('artifact registry header mismatch')
promote = {
    'PLAN.md', 'docs/design.md', 'docs/architecture', 'docs/spec/README.md',
    'docs/spec/interaction-core.md', 'docs/spec/planning.md', 'docs/spec/binding-spi.md',
    'docs/spec/v5-authority-reset.toml', 'docs/ADRs', 'docs/api-ownership.csv',
    'docs/requirements.csv', 'docs/work-packages/index.toml',
    'docs/work-packages/WP-100-core.md', 'docs/work-packages/WP-200-planning.md',
    'docs/work-packages/WP-300-bindings.md', 'docs/work-packages/WP-400-servient.md',
    'tools/design-check/Cargo.toml', 'tools/check-design-artifacts.sh',
    'tools/check-architecture-adrs.sh',
}
seen = set()
for row in rows[1:]:
    if row[0] in promote:
        row[3] = '5.1'
        seen.add(row[0])
if seen != promote:
    raise SystemExit(f'artifact promotion mismatch missing={sorted(promote-seen)}')
if not any(row[0] == 'tools/check-v5.1-authority.py' for row in rows[1:]):
    rows.append(['tools/check-v5.1-authority.py','active-v5.1-authority-checker',
                 'non-normative-checker','5.1','1','docs/design.md'])
out = io.StringIO()
csv.writer(out, lineterminator='\n').writerows(rows)
registry_path.write_text(out.getvalue(), encoding='utf-8')

# Normal design checker selects active v5.1 and freezes review provenance.
path = 'tools/design-check/src/main.rs'
text = read(path)
if text.count('"5.0"') != 2:
    raise SystemExit('design-check expected exactly two v5.0 selectors')
text = text.replace('"5.0"', '"5.1"')
anchor = '    require_root_string(&document, "status", "active", relative)?;\n'
addition = anchor + f'''    require_root_string(
        &document,
        "decision",
        "docs/ADRs/0019-consumer-one-shot-authority-entry.org",
        relative,
    )?;
    require_root_string(
        &document,
        "workspace_basis",
        "workspace/0061-consumer-one-shot-domain-entry.md",
        relative,
    )?;
    require_root_string(
        &document,
        "reviewed_candidate_commit",
        "{REVIEWED}",
        relative,
    )?;
'''
if text.count(anchor) != 1:
    raise SystemExit('design-check authority status anchor mismatch')
text = text.replace(anchor, addition, 1)
write(path, text)

# Candidate routing is replaced by normal active-v5.1 validation.
check_design = '''#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
registry="$root/docs/artifacts.csv"

expected_header='path,role,normativity,design_revision,schema_version,requirement_source'
if [[ $(head -n 1 "$registry") != "$expected_header" ]]; then
    echo "design artifact check: invalid artifact registry header" >&2
    exit 1
fi

awk -F, '\''
    NF != 6 {
        printf "design artifact check: line %d has %d columns; expected 6\\n", NR, NF > "/dev/stderr"
        bad = 1
    }
    NR > 1 && seen[$1]++ {
        printf "design artifact check: duplicate path: %s\\n", $1 > "/dev/stderr"
        bad = 1
    }
    END { exit bad }
'\'' "$registry"

while IFS=, read -r relative _role _normativity _revision _schema requirement_source; do
    [[ "$relative" == "path" ]] && continue
    if [[ "$relative" = /* || "$relative" == *".."* || ! -e "$root/$relative" ]]; then
        echo "design artifact check: invalid or missing registered path: $relative" >&2
        exit 1
    fi
    if [[ "$requirement_source" = /* || "$requirement_source" == *".."* || ! -e "$root/$requirement_source" ]]; then
        echo "design artifact check: invalid or missing requirement source: $requirement_source" >&2
        exit 1
    fi
done <"$registry"

python3 "$root/tools/check-v5.1-authority.py"
cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- check
"$root/tools/check-api-ownership.sh"
"$root/tools/check-architecture-adrs.sh"
"$root/tools/check-directory-client-scope.sh"
"$root/tools/check-resource-limits.sh"
"$root/tools/check-legacy-api-absence.sh"
cargo run --locked --quiet --manifest-path "$root/tools/performance-harness/Cargo.toml" -- verify

echo "design artifact check: active v5.1 authority and stable cross-cutting invariants validated"
'''
write('tools/check-design-artifacts.sh', check_design)

# Accepted ADR checker now enforces ADR-0019 projection.
path = 'tools/check-architecture-adrs.sh'
text = read(path)
anchor = "    'docs/design.md|ADR-0018' " + "\\" + "\n"
if text.count(anchor) != 1:
    raise SystemExit('architecture ADR projection anchor mismatch')
text = text.replace(anchor, anchor + "    'docs/design.md|ADR-0019' " + "\\" + "\n", 1)
write(path, text)

# Workspace decision is fully migrated.
replace_once('workspace/0061-consumer-one-shot-domain-entry.md', 'Status: DECIDED', 'Status: MIGRATED')

# Candidate-only projection/checkers disappear from the active revision.
for stale in [
    ROOT / 'tools/check-v5.1-authority-candidate.py',
    ROOT / 'tools/check-architecture-adrs-candidate.sh',
]:
    if not stale.exists():
        raise SystemExit(f'missing expected candidate-only file {stale}')
    stale.unlink()

# Temporary transport files are removed from the resulting activation diff.
for temp in [
    ROOT / 'tools/apply-v5.1-activation.py',
    ROOT / '.github/workflows/tmp-v5.1-activation-runner.yml',
]:
    if temp.exists():
        temp.unlink()
