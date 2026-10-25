#!/bin/sh
# Compiles the example play with the class in draft and final mode and checks
# the scene casts the class wrote. Run from anywhere: tex/test/run.sh
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$root"/tex/*.cls "$root"/tex/*.sty "$root/examples/ornek-oyun.tex" "$work/"
cd "$work"
printf '\\PassOptionsToClass{final}{tiyatro}\\input{ornek-oyun}\n' > final.tex

for doc in ornek-oyun final; do
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
