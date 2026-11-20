#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

portable="${RYUJINX_PORTABLE:-/mnt/c/Emulateurs/Ryujinx_Modding/portable}"
plugins="$portable/sdcard/atmosphere/contents/01006A800016E000/romfs/skyline/plugins"

if [[ ! -d "$plugins" || ! -f "$plugins/libarcropolis.nro" ]]; then
  echo "Erreur : dossier Skyline/ARCropolis introuvable dans le Ryujinx portable : $plugins" >&2
  exit 1
fi

cargo skyline build --release

mapfile -d '' nros < <(find target -type f -path '*/release/*' -name '*remove_the_frame*.nro' -print0)
if (( ${#nros[@]} != 1 )); then
  echo "Erreur : ${#nros[@]} NRO correspondants trouves ; attendu : 1." >&2
  exit 1
fi

source_nro="${nros[0]}"
destination="$plugins/libremove_the_frame.nro"

if [[ -f "$destination" ]] && ! cmp -s -- "$source_nro" "$destination"; then
  mkdir -p .deploy-backups
  backup=".deploy-backups/libremove_the_frame_$(date -u +%Y%m%dT%H%M%SZ)_$$.nro.bak"
  cp -p -- "$destination" "$backup"
  printf 'Ancienne version sauvegardee : %s\n' "$backup"
fi

cp -- "$source_nro" "$destination"
cmp -s -- "$source_nro" "$destination"
printf 'NRO installe : %s\n' "$destination"
sha256sum -- "$destination"
