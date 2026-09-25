#!/usr/bin/env bash
# Downloads FamilySearch's official GEDCOM 7 test files into target/gedcom-testfiles/.
#
# They are fetched rather than committed: the files are published for testing implementations
# and carry no licence that permits redistributing them, so this repository only ever holds the
# list of their names and the revision they were taken from.
#
# Pinned to one revision of FamilySearch/GEDCOM.io so that a test result can be reproduced. To
# move to a newer set, change REVISION and check what the new files exercise.

set -euo pipefail

REVISION="10cc419afad82184e2a89e5883169e4b4349a444"
BASE="https://raw.githubusercontent.com/FamilySearch/GEDCOM.io/${REVISION}/testfiles/gedcom70"
FILES=(
    age.ged
    escapes.ged
    filename-1.ged
    lang.ged
    long-url.ged
    maximal70-tree1.ged
    maximal70-tree2.ged
    maximal70.ged
    minimal70.ged
    notes-1.ged
    obje-1.ged
    remarriage1.ged
    remarriage2.ged
    same-sex-marriage.ged
    voidptr.ged
    xref.ged
)

root="$(cd "$(dirname "$0")/.." && pwd)"
target="${root}/target/gedcom-testfiles"
mkdir -p "$target"

for file in "${FILES[@]}"; do
    # Three attempts: raw.githubusercontent.com drops the occasional connection, and one flaky
    # request should not fail a CI run.
    for attempt in 1 2 3; do
        if curl --fail --silent --show-error --location --max-time 60 \
            --output "${target}/${file}" "${BASE}/${file}"; then
            break
        fi
        if [ "$attempt" = 3 ]; then
            echo "could not fetch ${file}" >&2
            exit 1
        fi
        sleep 2
    done
done

echo "Fetched ${#FILES[@]} files from FamilySearch/GEDCOM.io@${REVISION:0:7} into ${target}"
