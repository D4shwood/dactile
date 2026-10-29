#!/bin/sh
# Compiles the example play with the class in draft and final mode (and with
# the director's notes left out), a new play with nobody in it yet, and checks
# the scene casts the class wrote, the rerun warning when they were stale, and
# a run with the casts seeded as Dactile does, and comments on # headings.
# Run from anywhere: tex/test/run.sh
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

# A comment on a # heading ends there: Dactile's scene ids (`%@k3`).
printf '\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\title{Yeni}\n\\begin{document}\n#perde Bir %%@a1\n\n#sahne İç \\%% yarı — Gece %%@k3\n\n\\se Merhaba.\n\n#sahne[2.5] Dış %% bir not {x}\n\\se Tekrar.\n\\end{document}\n' > ids.tex
xelatex -interaction=nonstopmode -halt-on-error ids.tex > /dev/null 2>&1 || { grep -A6 '^! ' ids.log; echo "FAIL: ids"; exit 1; }
pdftotext ids.pdf - | grep -q 'İç % yarı — Gece$' || { pdftotext ids.pdf - | head -20; echo "FAIL: heading with an id"; exit 1; }
pdftotext ids.pdf - | grep -q 'Sahne 2.5 – Dış$' || { echo "FAIL: heading with a comment"; exit 1; }
! pdftotext ids.pdf - | grep -q '@k3\|not' || { echo "FAIL: a comment printed"; exit 1; }
echo "ok: comments on headings"

# Entrances and exits: printed as directions, the characters in the scene.
printf '\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\karakter{OZAN}{oz}\n\\karakter{İHTİYAR}{ih}\n\\begin{document}\n#sahne Ev\n\\girer{se}\n\\se Merhaba.\n\\girer{oz,ih}\n\\cikar{se}\n\\end{document}\n' > moves.tex
for run in 1 2; do
    xelatex -interaction=nonstopmode -halt-on-error moves.tex > /dev/null 2>&1 || { grep -A6 '^! ' moves.log; echo "FAIL: moves"; exit 1; }
done
pdftotext moves.pdf - > moves.txt
grep -q '(SELİN girer.)' moves.txt || { cat moves.txt; echo "FAIL: \\girer"; exit 1; }
grep -q '(OZAN ile İHTİYAR girerler.)' moves.txt || { cat moves.txt; echo "FAIL: two enter"; exit 1; }
grep -q '(SELİN çıkar.)' moves.txt || { cat moves.txt; echo "FAIL: \\cikar"; exit 1; }
grep -qxF '\tiyatrocast{-1}{se,oz,ih}' moves.aux || { cat moves.aux; echo "FAIL: moves cast"; exit 1; }
echo "ok: entrances and exits"
