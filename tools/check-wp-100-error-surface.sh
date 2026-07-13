#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

source_roots=(".")
public_docs=(".")
public_doc_globs=(
    --glob '*.md'
    --glob '!docs/amendments/**'
    --glob '!docs/deprecated/**'
)

if rg -n --glob '*.rs' '\bSecurityError\b' "${source_roots[@]}" \
    || rg -n "${public_doc_globs[@]}" '\bSecurityError\b' "${public_docs[@]}"; then
    echo "WP-100 error surface check: legacy SecurityError remains" >&2
    exit 1
fi

for variant in \
    UnknownAffordance UnsupportedBinding Transport InvalidInteraction \
    MissingHandler InboundDispatch HandlerPanic Timeout TimeoutUnsupported \
    UnsupportedForm ContentTypeMismatch; do
    if rg -n --glob '*.rs' "CoreError::$variant\\b" "${source_roots[@]}" \
        || rg -n "${public_doc_globs[@]}" "CoreError::$variant\\b" "${public_docs[@]}"; then
        echo "WP-100 error surface check: legacy CoreError::$variant remains" >&2
        exit 1
    fi
done

# These unique legacy names must also be absent from public documentation. The
# normative migration amendments retain them intentionally in mapping tables.
legacy_names='\b(InvalidInteraction|MissingHandler|InboundDispatch|HandlerPanic|TimeoutUnsupported|ContentTypeMismatch)\b'
if rg -n --glob '*.rs' "$legacy_names" "${source_roots[@]}"; then
    echo "WP-100 error surface check: legacy error name remains in source" >&2
    exit 1
fi
if rg -n "${public_doc_globs[@]}" "$legacy_names" "${public_docs[@]}"; then
    echo "WP-100 error surface check: legacy error name remains in public documentation" >&2
    exit 1
fi

# Protocol-private predicates remain allowed, so documentation evidence uses
# the removed ServientError-qualified path rather than rejecting a naked name.
for predicate in is_missing_handler is_security is_timeout; do
    if rg -n --glob '*.rs' "\\b$predicate\\b" "servient" "clinkz-wot" \
        || rg -n "${public_doc_globs[@]}" "ServientError::$predicate\\b" "${public_docs[@]}"; then
        echo "WP-100 error surface check: legacy Servient predicate $predicate remains" >&2
        exit 1
    fi
done

echo "WP-100 error surface check: legacy error APIs absent"
