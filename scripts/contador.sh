#!/usr/bin/env bash
# Muestra el avance del contador de commits frente al calendario de la semana.
# Uso: scripts/contador.sh [rama]   (por defecto, develop)
set -euo pipefail

rama="${1:-develop}"
meta_total=1000

# Fecha de cierre de cada dia y acumulado esperado a ese cierre (docs/GIT-WORKFLOW.md, 1.1).
dias=("2026-10-16:160" "2026-10-17:300" "2026-10-18:440" "2026-10-19:580" "2026-10-20:720" "2026-10-21:860" "2026-10-22:1000")

actuales=$(git rev-list --count "$rama")
hoy=$(date +%F)

# Antes de la semana la meta es la base (17); durante ella, el acumulado del dia en curso;
# despues, la meta total.
meta_hoy=17
if [[ "$hoy" > "${dias[0]%%:*}" || "$hoy" == "${dias[0]%%:*}" ]]; then
  meta_hoy=$meta_total
  for d in "${dias[@]}"; do
    if [[ "$hoy" < "${d%%:*}" || "$hoy" == "${d%%:*}" ]]; then
      meta_hoy="${d##*:}"
      break
    fi
  done
fi

faltan_total=$(( meta_total - actuales ))
(( faltan_total < 0 )) && faltan_total=0
diferencia=$(( actuales - meta_hoy ))

printf 'Rama:               %s\n' "$rama"
printf 'Commits actuales:   %d de %d\n' "$actuales" "$meta_total"
printf 'Faltan:             %d\n' "$faltan_total"
printf 'Meta al cierre hoy: %d (%s)\n' "$meta_hoy" "$hoy"
if (( diferencia >= 0 )); then
  printf 'Estado:             al dia, %d por delante\n' "$diferencia"
else
  printf 'Estado:             %d por detras del calendario\n' "$(( -diferencia ))"
fi
