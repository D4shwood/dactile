#!/bin/sh
# Makes the folder of programs and models that lets Dactile read scanned
# books with nothing else installed: tesseract and poppler's tools with the
# libraries they need, and the models. Usage: packaging/ocr-bundle.sh DIR
# Needs tesseract-ocr and poppler-utils on the machine that builds.
set -eu
dest=$1
models=https://raw.githubusercontent.com/tesseract-ocr/tessdata_best/e12c65a915945e4c28e237a9b52bc4a8f39a0cec
mkdir -p "$dest/bin" "$dest/lib" "$dest/tessdata"

for program in tesseract pdftoppm pdftotext pdfinfo; do
    cp -L "$(command -v "$program")" "$dest/bin/"
    # Not glibc's own: those must be the system's.
    ldd "$dest/bin/$program" | awk '/=> \//{print $3}' |
        grep -Ev '/(libc|libm|libdl|libpthread|librt|libresolv|ld-linux[^/]*)\.so' |
        while read -r lib; do cp -Ln "$lib" "$dest/lib/" 2>/dev/null || true; done
done

# The font of the words-only PDFs it reads pages through.
cp "$(find /usr/share -name pdf.ttf | head -1)" "$dest/tessdata/"

fetch() {
    [ -f "$dest/tessdata/$2" ] && [ "$(sha256sum "$dest/tessdata/$2" | cut -d' ' -f1)" = "$3" ] && return
    curl -fsSL -o "$dest/tessdata/$2" "$models/$1"
    echo "$3  $dest/tessdata/$2" | sha256sum -c -
}
fetch tur.traineddata tur.traineddata e0c3338dc17503dc7d335a507c9ae01b2b46cfd07561171e1e1ac55d85e8e438
fetch script/Latin.traineddata Latin.traineddata cf7216998bf7f363f18e3def04cc6b28ae954d2a5a3bdad09f743c9d3538ef35

cat > "$dest/README" <<'T'
Tesseract (Apache-2.0) and its tessdata_best models (Apache-2.0), and Poppler's
tools (GPL-2.0-or-later), with the libraries they use, each under its own
licence. Their sources: github.com/tesseract-ocr, poppler.freedesktop.org.
T
du -sh "$dest"
