#!/bin/sh
# Compiles the example play with the class in draft and final mode (and with
# the director's notes left out), a new play with nobody in it yet, and checks
# the scene casts the class wrote, the rerun warning when they were stale, and
# a run with the casts seeded as Dactile does, comments on # headings,
# entrances and exits, actor copies, and a play with others and footnotes.
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

# Those the list doesn't name, under it; a footnote in a line, in a direction
# and in headings, not in the contents.
printf '\\documentclass{tiyatro}\n\\title{Yeni}\n\\karakter[Bir kız]{SELİN}{se}\n\\digerkisiler{Köylüler, çobanlar.}\n\\begin{document}\n\\maketitle\n\\tableofcontents\n\\kisiler\n\n#perde Bir\\dipnot{Perde notu.}\n\n#sahne İç\\dipnot{Yer notu.}\n\n\\action{Işıklar\\dipnot{Sahne notu.}}\n\n\\se Buyurun\\dipnot{Eski bir söz.} efendim.\n\n#sahne Dış\\dipnot{}\n\n\\se Evet.\n\n\\end{document}\n' > others.tex

for doc in ornek-oyun final nonotes empty others; do
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

pdftotext -layout others.pdf others.txt
grep -q 'Köylüler, çobanlar\.' others.txt || { echo "FAIL: others missing"; cat others.txt; exit 1; }
grep -q 'Eski bir söz\.' others.txt || { echo "FAIL: footnote in a line missing"; cat others.txt; exit 1; }
grep -q 'Sahne notu\.' others.txt || { echo "FAIL: footnote in a direction missing"; cat others.txt; exit 1; }
grep -q 'Perde notu\.' others.txt || { echo "FAIL: footnote in an act's heading missing"; cat others.txt; exit 1; }
grep -q 'Yer notu\.' others.txt || { echo "FAIL: footnote in a scene's heading missing"; cat others.txt; exit 1; }
grep -q 'contentsline.*İç' others.toc || { echo "FAIL: heading missing from the contents"; cat others.toc; exit 1; }
if grep -q 'notu' others.toc; then echo "FAIL: footnote in the contents"; cat others.toc; exit 1; fi
echo "ok: others and footnotes"

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

# An actor's copy: their lines in bold, a marked entrance in bold, scenes
# without them only headings; the casts as seeded, no rerun asked for.
printf '\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\karakter{OZAN}{oz}\n\\title{Deneme}\n\\tiyatrocopy{se}{Deniz Kaya}\n\\tiyatrocopycover{I-1 (s.~\\pageref{tiy@I-1})}\n\\begin{document}\n\\maketitle\n#perde Bir\n#sahne Ev\n\\tiyatromark\\girer{se}\n\\se Merhaba.\n\\oz \\tiyatrogap{3}\n\\sedo Oturur.\n\\stagedir{Işık.}\n#sahne Yol\n\\end{document}\n' > copy.tex
printf '\\relax\n\\tiyatrocast{I-1}{se,oz}\n\\tiyatrocast{I-2}{oz}\n' > copy.aux
for run in 1 2; do
    xelatex -interaction=nonstopmode -halt-on-error copy.tex > /dev/null 2>&1 || { grep -A6 '^! ' copy.log; echo "FAIL: copy"; exit 1; }
done
pdftotext copy.pdf - > copy.txt
grep -q 'Deniz Kaya' copy.txt || { cat copy.txt; echo "FAIL: copy cover"; exit 1; }
grep -q 'I-1 (s. 1)' copy.txt || { cat copy.txt; echo "FAIL: copy cover page"; exit 1; }
grep -q 'SELİN, OZAN' copy.txt || { cat copy.txt; echo "FAIL: copy kept the seeded cast"; exit 1; }
! grep -q 'Rerun to get them right' copy.log || { echo "FAIL: copy asks for a rerun"; exit 1; }
echo "ok: an actor's copy"

# Names over lines in capitals; the name column holds them in bold, even when
# theirs is the widest.
printf '\\documentclass{tiyatro}\n\\karakter{Arabacı}{ar}\n\\karakter{Jonas}{jo}\n\\title{Deneme}\n\\tiyatrocopy{ar}{}\n\\begin{document}\n#perde Bir\n#sahne Yol\n\\ar Merhaba Jonas!\n\\jo Merhaba.\n\\end{document}\n' > wide.tex
xelatex -interaction=nonstopmode -halt-on-error wide.tex > /dev/null 2>&1 || { grep -A6 '^! ' wide.log; echo "FAIL: wide"; exit 1; }
pdftotext wide.pdf wide.txt
grep -q 'ARABACI' wide.txt || { cat wide.txt; echo "FAIL: names over lines in capitals"; exit 1; }
! grep -q 'ARABACIMerhaba' wide.txt || { cat wide.txt; echo "FAIL: a bold name runs into its line"; exit 1; }
echo "ok: bold names fit"

# \tiyatronoend: no curtain at the end.
printf '\\documentclass{tiyatro}\n\\tiyatronoend\n\\begin{document}\nTablo.\n\\end{document}\n' > noend.tex
xelatex -interaction=nonstopmode -halt-on-error noend.tex > /dev/null 2>&1 || { grep -A6 '^! ' noend.log; echo "FAIL: noend"; exit 1; }
! pdftotext noend.pdf - | grep -q 'Son' || { echo "FAIL: noend printed the end"; exit 1; }
echo "ok: no end"

# #önsahne: an unnumbered prologue; the next scene is still Sahne 1.
printf '\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\begin{document}\n#perde Bir\n#önsahne Karanlık\n\\se Önce.\n#sahne Ev\n\\se Sonra.\n\\end{document}\n' > pro.tex
xelatex -interaction=nonstopmode -halt-on-error pro.tex > /dev/null 2>&1 || { grep -A6 '^! ' pro.log; echo "FAIL: prologue"; exit 1; }
pdftotext pro.pdf pro.txt
grep -q 'Ön Sahne' pro.txt && grep -q 'Sahne 1' pro.txt || { cat pro.txt; echo "FAIL: prologue numbering"; exit 1; }
grep -q 'tiyatrocast{I-0}{se}' pro.aux || { echo "FAIL: prologue cast key"; exit 1; }
echo "ok: prologue"

# \scenetitleflush: the side of the scene headings; a wrong word stops the build.
flush() {
    printf '\\documentclass{tiyatro}\n\\scenetitleflush%s\n\\begin{document}\n#sahne Kıyı\n\nTablo.\n\\end{document}\n' "$2" > "flush-$1.tex"
}
at() {
    pdftotext -bbox "flush-$1.pdf" - | sed -n 's/.*xMin="\([0-9.]*\)".*>Kıyı<.*/\1/p' | head -1
}
for side in left sol sağ right center; do
    flush "$side" "[$side]"
    xelatex -interaction=nonstopmode -halt-on-error "flush-$side.tex" > /dev/null 2>&1 || { grep -A6 '^! ' "flush-$side.log"; echo "FAIL: flush $side"; exit 1; }
done
awk -v l="$(at left)" -v c="$(at center)" -v r="$(at right)" -v s="$(at sol)" -v g="$(at sağ)" \
    'BEGIN { exit !(l < c && c < r && l == s && r == g) }' || { echo "FAIL: flush sides"; exit 1; }
flush wrong '[ust]'
! xelatex -interaction=nonstopmode -halt-on-error flush-wrong.tex > /dev/null 2>&1 || { echo "FAIL: flush took a wrong word"; exit 1; }
echo "ok: scene heading side"

# The cover: its own page; only what the play fills in; an editor under the author.
printf '\\documentclass{tiyatro}\n\\title{Kar}\n\\author{Örnek Yazar}\n\\editedby{Örnek Kolektif}\n\\copyrightnotice{}\n\\address{}\n\\email{}\n\\telephone{}\n\\karakter[Bir kız]{SELİN}{se}\n\\begin{document}\n\\maketitle\n\\kisiler\n\n#sahne Kıyı\n\n\\se Evet.\n\n\\end{document}\n' > cover.tex
xelatex -interaction=nonstopmode -halt-on-error cover.tex > /dev/null 2>&1 || { grep -A6 '^! ' cover.log; echo "FAIL: cover"; exit 1; }
pdftotext -f 1 -l 1 cover.pdf cover1.txt
grep -q 'Düzenleyen: Örnek Kolektif' cover1.txt || { cat cover1.txt; echo "FAIL: editor"; exit 1; }
if grep -q '©' cover1.txt; then echo "FAIL: an empty line on the cover"; exit 1; fi
if grep -qE 'KİŞİLER|SELİN|Kıyı' cover1.txt; then cat cover1.txt; echo "FAIL: something else on the cover"; exit 1; fi
echo "ok: cover"

# The cast page is its own; \kisiler* lets the play go on under it.
[ "$(pdfinfo cover.pdf | sed -n 's/^Pages: *//p')" = 3 ] || { echo "FAIL: cast page not its own"; exit 1; }
sed 's/\\kisiler/\\kisiler*/' cover.tex > flow.tex
xelatex -interaction=nonstopmode -halt-on-error flow.tex > /dev/null 2>&1 || { grep -A6 '^! ' flow.log; echo "FAIL: flow"; exit 1; }
[ "$(pdfinfo flow.pdf | sed -n 's/^Pages: *//p')" = 2 ] || { echo "FAIL: kisiler* broke the page"; exit 1; }
echo "ok: cast page"

# \dramatispersonae, in a Turkish play too.
sed 's/\\begin{document}/\\dramatispersonae\n&/' cover.tex > latin.tex
xelatex -interaction=nonstopmode -halt-on-error latin.tex > /dev/null 2>&1 || { grep -A6 '^! ' latin.log; echo "FAIL: latin"; exit 1; }
pdftotext latin.pdf - | grep -qi 'Dramatis' || { echo "FAIL: no latin title"; exit 1; }
if pdftotext latin.pdf - | grep -qi 'KİŞİLER'; then echo "FAIL: both titles"; exit 1; fi
echo "ok: dramatis personae"

# \coverstyle and \caststyle; a wrong word stops the build.
sed 's/\\begin{document}/\\coverstyle{title}{align=left, size=large, style=bold, before=1cm, after=2cm}\n\\caststyle{title}{align=right, style=italic}\n\\caststyle{names}{align=right}\n&/' cover.tex > styled.tex
xelatex -interaction=nonstopmode -halt-on-error styled.tex > /dev/null 2>&1 || { grep -A6 '^! ' styled.log; echo "FAIL: styled"; exit 1; }
xmin() { pdftotext -f "$2" -l "$2" -bbox "$1" - | sed -n "s/.*xMin=\"\([0-9.]*\)\".*>$3<.*/\1/p" | head -1; }
awk -v a="$(xmin styled.pdf 1 Kar)" -v b="$(xmin cover.pdf 1 Kar)" 'BEGIN { exit !(a < b) }' || { echo "FAIL: coverstyle align"; exit 1; }
for bad in 'align=up' 'size=big' 'style=wavy' 'colour=red'; do
    sed "s/\\\\begin{document}/\\\\coverstyle{title}{$bad}\n&/" cover.tex > wrong.tex
    ! xelatex -interaction=nonstopmode -halt-on-error wrong.tex > /dev/null 2>&1 || { echo "FAIL: took $bad"; exit 1; }
done
sed 's/\\begin{document}/\\caststyle{nothing}{size=large}\n&/' cover.tex > wrong.tex
! xelatex -interaction=nonstopmode -halt-on-error wrong.tex > /dev/null 2>&1 || { echo "FAIL: took a part that isn't there"; exit 1; }
echo "ok: styles"

# A secret character, under all its names.
printf '\\documentclass{tiyatro}\n\\sirkarakter{HERKES}{hep}\n\\secretcharacter{SEYİRCİ}{sey}\n\\gizlikarakter{KORO}{kor}\n\\begin{document}\n\\hep Evet.\n\n\\sey Hayır.\n\n\\kor Belki.\n\n\\end{document}\n' > secret.tex
xelatex -interaction=nonstopmode -halt-on-error secret.tex > /dev/null 2>&1 || { grep -A6 '^! ' secret.log; echo "FAIL: secret"; exit 1; }
echo "ok: secret characters"

# A draft is set in the final copy's font.
pdffonts ornek-oyun.pdf > fonts.txt
pdffonts final.pdf > fonts-final.txt
[ "$(sed -n 's/^[A-Z]*+\([^ -]*\).*/\1/p' fonts.txt | sort -u | grep -v Mono)" = "$(sed -n 's/^[A-Z]*+\([^ -]*\).*/\1/p' fonts-final.txt | sort -u | grep -v Mono)" ] || { cat fonts.txt fonts-final.txt; echo "FAIL: draft font"; exit 1; }
echo "ok: draft font"

# \playfont: the PDF takes it; one that isn't there only warns.
sed 's/\\begin{document}/\\playfont{DejaVu Serif}\n&/' cover.tex > font.tex
xelatex -interaction=nonstopmode -halt-on-error font.tex > /dev/null 2>&1 || { grep -A6 '^! ' font.log; echo "FAIL: playfont"; exit 1; }
pdffonts font.pdf | grep -q 'DejaVuSerif' || { pdffonts font.pdf; echo "FAIL: playfont not used"; exit 1; }
sed 's/\\begin{document}/\\playfont{No Such Font}\n&/' cover.tex > nofont.tex
xelatex -interaction=nonstopmode -halt-on-error nofont.tex > /dev/null 2>&1 || { grep -A6 '^! ' nofont.log; echo "FAIL: a missing font stopped the build"; exit 1; }
grep -q 'No Such Font' nofont.log || { echo "FAIL: no warning for a missing font"; exit 1; }
echo "ok: play font"

# \playfontsize: the text grows; a wrong word stops the build.
body=$(for i in $(seq 60); do printf 'Karlar yağdı, yollar kapandı, değirmen sustu ve köy uykuya daldı.\n\n'; done)
printf '\\documentclass[final]{tiyatro}\n\\begin{document}\n%s\n\\end{document}\n' "$body" > plain.tex
sed 's/\\begin{document}/\\playfontsize{20}\n&/' plain.tex > size.tex
xelatex -interaction=nonstopmode -halt-on-error plain.tex > /dev/null 2>&1 || { grep -A6 '^! ' plain.log; echo "FAIL: plain"; exit 1; }
xelatex -interaction=nonstopmode -halt-on-error size.tex > /dev/null 2>&1 || { grep -A6 '^! ' size.log; echo "FAIL: playfontsize"; exit 1; }
[ "$(pdfinfo size.pdf | sed -n 's/^Pages: *//p')" -gt "$(pdfinfo plain.pdf | sed -n 's/^Pages: *//p')" ] || { echo "FAIL: playfontsize changed nothing"; exit 1; }
sed 's/\\begin{document}/\\playfontsize{big}\n&/' cover.tex > wrong.tex
! xelatex -interaction=nonstopmode -halt-on-error wrong.tex > /dev/null 2>&1 || { echo "FAIL: took a size that isn't one"; exit 1; }
echo "ok: play font size"

# \icindekiler: its own page after the cover, in the play's words; the play
# still starts on page 1. Without it, no such page.
sed 's/\\begin{document}/\\icindekiler\n&/' ornek-oyun.tex > contents.tex
for pass in 1 2; do xelatex -interaction=nonstopmode -halt-on-error contents.tex > /dev/null 2>&1 || { grep -A6 '^! ' contents.log; echo "FAIL: contents"; exit 1; }; done
pdftotext -f 2 -l 2 contents.pdf - | grep -q 'İçindekiler' || { echo "FAIL: no contents page"; exit 1; }
pdftotext -f 2 -l 2 contents.pdf - | grep -q 'Sahne 1' || { echo "FAIL: contents empty"; exit 1; }
if pdftotext -f 3 -l 3 contents.pdf - | grep -qi 'çindekiler\|ÇINDEKILER'; then echo "FAIL: contents in the next page's head"; exit 1; fi
if pdftotext -f 1 -l 1 contents.pdf - | grep -q 'İçindekiler'; then echo "FAIL: contents on the cover"; exit 1; fi
pdftotext -f 3 -l 3 contents.pdf - | grep -qx '1' || { echo "FAIL: the play doesn't start on page 1"; exit 1; }
if pdftotext ornek-oyun.pdf - | grep -q 'İçindekiler'; then echo "FAIL: contents nobody asked for"; exit 1; fi
sed 's/\\begin{document}/\\playcontents\n&/' ornek-oyun.tex > contents2.tex
xelatex -interaction=nonstopmode -halt-on-error contents2.tex > /dev/null 2>&1 || { echo "FAIL: playcontents"; exit 1; }
echo "ok: contents"
