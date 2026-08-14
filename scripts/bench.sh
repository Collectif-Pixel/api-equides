#!/usr/bin/env bash

set -euo pipefail

BASE="${1:-http://127.0.0.1:8080}"
CONCURRENCE="${CONCURRENCE:-50}"
REQUETES="${REQUETES:-20000}"

if ! command -v ab > /dev/null 2>&1; then
  echo "erreur : ApacheBench (ab) est introuvable." >&2
  echo "  macOS  : fourni avec le système" >&2
  echo "  Debian : sudo apt install apache2-utils" >&2
  exit 1
fi

if ! curl -sf "$BASE/healthz" > /dev/null; then
  echo "erreur : aucune instance ne répond sur $BASE" >&2
  echo "  lancez d'abord : make servir" >&2
  exit 1
fi

extraire() {
  python3 -c "
import sys, re
t = sys.stdin.read()
rps = re.search(r'Requests per second:\s+([\d.]+)', t)
p50 = re.search(r'\n\s+50%\s+(\d+)', t)
p99 = re.search(r'\n\s+99%\s+(\d+)', t)
non2 = re.search(r'Non-2xx responses:\s+(\d+)', t)
print(f\"{float(rps.group(1)):>12,.0f} req/s   p50={p50.group(1):>4} ms  p99={p99.group(1):>4} ms  non-2xx={non2.group(1) if non2 else 0}\")
"
}

mesurer() {
  printf '%-44s' "$1"
  shift
  ab -k -c "$CONCURRENCE" -n "$REQUETES" -q "$@" 2> /dev/null | extraire
}

ID=$(curl -s "$BASE/v1/equides?limite=1" \
  | python3 -c "import json,sys; print(json.load(sys.stdin)['donnees'][0]['id'])")
ETAG=$(curl -s -D- -o /dev/null "$BASE/v1/equides?limite=20" \
  | grep -i '^etag:' | tr -d '\r' | cut -d' ' -f2)

echo "cible : $BASE   concurrence=$CONCURRENCE requêtes=$REQUETES"
echo
printf '%-44s%s\n' "scénario" "débit et latences"
printf '%s\n' "--------------------------------------------------------------------------------------"

mesurer "page par défaut (limite=20)"        "$BASE/v1/equides?limite=20"
mesurer "consultation par identifiant"       "$BASE/v1/equides/$ID"
mesurer "recherche par nom"                  "$BASE/v1/equides?nom=qabalah"
mesurer "filtre multi-critères"              "$BASE/v1/equides?race=Trotteur%20Francais&sexe=Femelle&annee_naissance=2015"
mesurer "page 200 lignes + tri global"       "$BASE/v1/equides?limite=200&tri=-annee"
mesurer "répartition sur tout le fichier"    "$BASE/v1/stats/repartition?dimension=race"
mesurer "pedigree 5 générations"             "$BASE/v1/equides/$ID/pedigree?generations=5"
mesurer "revalidation 304"                   -H "If-None-Match: $ETAG" "$BASE/v1/equides?limite=20"
printf '%s\n' "--------------------------------------------------------------------------------------"
mesurer "/healthz (plafond de l'injecteur)"  "$BASE/healthz"
echo
echo "Un endpoint proche du débit de /healthz signale un banc saturé, pas un service saturé."
