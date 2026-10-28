#!/bin/sh
# Compiles the example play with the class in draft and final mode (and with
# the director's notes left out), a new play with nobody in it yet, and checks
# the scene casts the class wrote, the rerun warning when they were stale, and
# a run with the casts seeded as Dactile does. Run from anywhere: tex/test/run.sh
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$root"/tex/*.cls "$root"/tex/*.sty "$root/examples/ornek-oyun.tex" "$work/"
cd "$work"
printf '\\PassOptionsToClass{final}{tiyatro}\\input{ornek-oyun}\n' > final.tex
printf '\\PassOptionsToClass{nonotes}{tiyatro}\\input{ornek-oyun}\n' > nonotes.tex
# What `dactile new` writes: no characters yet, # headings.
printf '\\documentclass{tiyatro}\n\\title{Yeni}\n\\begin{document}\n\\maketitle\n\\kisiler\n\n#perde Bir\n\n#sahne İç — Gece\n\n\\action{Işıklar.}\n\n\\end{document}\n' > empty.tex

for doc in ornek-oyun final nonotes empty; do
    # Twice: the scene casts come from the first run's .aux file.
    for run in 1 2; do
        if ! xelatex -interaction=nonstopmode -halt-on-error "$doc.tex" > "$doc.out" 2>&1; then
            grep -A12 '^! ' "$doc.log" || tail -40 "$doc.out"
            echo "FAIL: $doc (run $run)"
            exit 1
        fi
    done
    echo "ok: $doc.pdf"
done

expect() {
    grep -qxF "$1" ornek-oyun.aux || { echo "FAIL: .aux lacks $1"; cat ornek-oyun.aux; exit 1; }
}
expect '\tiyatrocast{I-1}{se,oz,ih}'
expect '\tiyatrocast{I-2}{ka,co,hep,se}'
expect '\tiyatrocast{II-1}{oz,ih}'
echo "ok: scene casts"

# The first run had no casts: it must have asked for a rerun; the second not.
rm -f ornek-oyun.aux
xelatex -interaction=nonstopmode -halt-on-error ornek-oyun.tex > /dev/null 2>&1
grep -q 'Rerun to get them right' ornek-oyun.log || { echo "FAIL: no rerun warning"; exit 1; }
xelatex -interaction=nonstopmode -halt-on-error ornek-oyun.tex > /dev/null 2>&1
! grep -q 'Rerun to get them right' ornek-oyun.log || { echo "FAIL: rerun warning on a right .aux"; exit 1; }
echo "ok: rerun warning"

# Seeded as Dactile seeds it: one run, no warning.
printf '\\relax\n\\tiyatrocast{I-1}{se,oz,ih}\n\\tiyatrocast{I-2}{ka,co,hep,se}\n\\tiyatrocast{II-1}{oz,ih}\n' > ornek-oyun.aux
xelatex -interaction=nonstopmode -halt-on-error ornek-oyun.tex > /dev/null 2>&1
! grep -q 'Rerun to get them right' ornek-oyun.log || { echo "FAIL: seeded run asks for a rerun"; exit 1; }
echo "ok: one pass when seeded"
