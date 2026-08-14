# API Équidés

[![CI](https://github.com/Collectif-Pixel/api-equides/actions/workflows/ci.yml/badge.svg)](https://github.com/Collectif-Pixel/api-equides/actions/workflows/ci.yml)

API REST ouverte sur les fiches publiques d'équidés de l'**IFCE** : **4 446 638 fiches**, avec généalogie et indices de performance, servies depuis la mémoire. Sans clé, sans compte.

```
GET /v1/equides?race=Trotteur%20Francais&sexe=Femelle&annee_naissance=2015
GET /v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw
GET /v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/pedigree?generations=5
GET /v1/stats/repartition?dimension=robe&race=Pur%20Sang
```

> **Service indépendant, non affilié à l'IFCE.** La donnée servie est une extraction datée ; les fiches en ligne font seules autorité.

**Documentation** : [docs.api-equides.org](https://docs.api-equides.org) · **Référence** : [`/openapi.json`](https://api-equides.org/openapi.json) · **Code** MIT · **Données** [CRPA L321-1](https://docs.api-equides.org/donnees/provenance/) · Aucune donnée à caractère personnel

## Démarrage

Prérequis : Rust ≥ 1.90, et l'extraction JSONL à la racine.

```sh
make donnees    # ingère chevaux.jsonl → data/equides.bin (~15 s)
make servir     # API sur 127.0.0.1:8081
```

Image de **601 Mio**, démarrage en **4,4 s**, **~1,1 Gio** résidents.

## Endpoints

| Chemin | Description |
|---|---|
| `/v1/equides` | Recherche paginée (résumés) |
| `/v1/search?q=` | Autocomplétion sur le nom, classée par pertinence |
| `/v1/equides/{id}` | Fiche complète |
| `/v1/equides/{id}/pedigree` | Ascendance sur N générations |
| `/v1/equides/{id}/descendance` | Produits directs |
| `/v1/referentiels/{dimension}` | Valeurs admises par les filtres |
| `/v1/stats` · `/v1/stats/repartition` | Agrégats et facettes |
| `/v1/meta` | Provenance, licence, anomalies |
| `/` | Document de découverte : routes servies et gabarits d'URL |
| `/openapi.json` | Description OpenAPI 3.1 |
| `/healthz` · `/readyz` | Sondes |
| `/metrics` | Prometheus — **port d'administration uniquement** |

Filtres combinables, insensibles à la casse et aux accents ; erreurs [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) ; `ETag` et `Cache-Control`. Le détail — filtres, tri, pagination, quotas — est dans les [guides](https://docs.api-equides.org/guides/premiers-pas/).

## Architecture

```
chevaux.jsonl (3,4 Gio) → equides-ingest → data/equides.bin (601 Mio)
                                                   ↓ equides-api
                                    Store en mémoire (~1,1 Gio)
                                    colonnes + bitmaps + graphe résolu
```

| Fichier | Rôle |
|---|---|
| `src/store/` | Colonnes, dictionnaires, bitmaps, permutations, descendance |
| `src/query.rs` | Filtres, tri, pagination |
| `src/text.rs` · `src/ids.rs` | Repli ASCII et index des noms · codec des identifiants |
| `src/snapshot.rs` | Format d'image binaire (v5, postcard) |
| `src/api/` · `src/openapi.rs` | Routes, cache, limitation de débit, erreurs · description |

Le jeu tient en mémoire : aucune base de données n'intervient, un filtre se résout par intersection de bitmaps. **La filiation est résolue à l'ingestion** — chaque identifiant de parent devient un indice de ligne, si bien qu'une ascendance sur cinq générations coûte 31 lectures de tableau. **Les index ne sont pas sérialisés** : les reconstruire coûte 3,8 s au démarrage, et supprime toute question de compatibilité de format.

## Développement

```sh
make verifier   # fmt + clippy + tests, ce que la CI exécute
make audit      # cargo audit + versions des dépendances
make bench      # banc de charge local
```

**134 tests.** Deux méritent d'être signalés, ayant chacun trouvé un vrai défaut : `les_deux_strategies_de_tri_donnent_le_meme_resultat` compare les deux chemins du tri à une implémentation naïve, et `les_parametres_documentes_sont_exactement_ceux_acceptes` ferme l'écart entre l'OpenAPI écrit à la main et le code.

Une collection [Bruno](https://www.usebruno.com/) est versionnée dans [`bruno/`](bruno/) — 22 requêtes, 66 assertions, à lancer contre une instance chargée avec le jeu réel.

Voir [CONTRIBUTING.md](CONTRIBUTING.md).
