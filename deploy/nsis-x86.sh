#!/bin/bash
# Tauri's Windows installer is a 32-bit NSIS one, with a 32-bit plugin of
# Tauri's own; Arch's makensis makes 64-bit installers alone. This puts the
# stubs and plugins of NSIS 3.11, as Tauri hands it out, beside Arch's
# makensis, and a `makensis` in ~/.cache/dactile-nsis/bin that uses them;
# deploy.sh puts that first on the PATH. Once is enough.
set -euo pipefail

url=https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
sum=c7d27f780ddb6cffb4730138cd1591e841f4b7edb155856901cdf5f214394fa1
home="$HOME/.cache/dactile-nsis"
dir="$home/nsis-3.11"

command -v /usr/bin/makensis >/dev/null || { echo "First: paru -S nsis" >&2; exit 1; }
if [ ! -d "$dir/Stubs" ]; then
  mkdir -p "$home/dl"
  curl -sSL -o "$home/dl/nsis-3.11.zip" "$url"
  echo "$sum  $home/dl/nsis-3.11.zip" | sha256sum -c --quiet
  unzip -q -o "$home/dl/nsis-3.11.zip" -d "$home"
fi
# makensis starts from its own (64-bit) stub, then the script's Target
# turns it to 32 bits: both are there.
cp -n /usr/share/nsis/Stubs/*-amd64-unicode "$dir/Stubs/"
cp -rn /usr/share/nsis/Plugins/amd64-unicode "$dir/Plugins/"
mkdir -p "$home/bin"
cat > "$home/bin/makensis" <<'EOF'
#!/bin/sh
NSISDIR="$HOME/.cache/dactile-nsis/nsis-3.11" exec /usr/bin/makensis -XTarget\ x86-unicode "$@"
EOF
chmod +x "$home/bin/makensis"
echo "makensis for Tauri's installer: $home/bin/makensis"
