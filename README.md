# API Équidés

API REST ouverte sur les fiches publiques d'équidés de l'**IFCE** : **4 446 638 fiches**, avec généalogie et indices de performance, servies depuis la mémoire.

```
GET /v1/equides?race=Trotteur%20Francais&sexe=Femelle&annee_naissance=2015
GET /v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw
GET /v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/pedigree?generations=5
GET /v1/stats/repartition?dimension=robe&race=Pur%20Sang
```

> **Service indépendant, non affilié à l'IFCE.** La donnée servie est une extraction datée ; les fiches en ligne font seules autorité.

**Code** sous licence MIT · **Données** réutilisées au titre de l'[article L321-1 du CRPA](https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000031366350/LEGISCTA000032255212/) · **Aucune donnée à caractère personnel**

---

## Démarrage

Prérequis : Rust ≥ 1.90, et l'extraction JSONL à la racine.

```sh
make donnees    # ingère chevaux.jsonl → data/equides.bin (~15 s)
make servir     # API sur 127.0.0.1:8081
```

Ingestion : image de **601 Mio**. Démarrage : **4,4 s**, puis **~1,1 Gio** résidents.

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

**Filtres** : `race`, `robe`, `sexe`, `discipline`, `indice`, `annee_naissance`, `nom`, `annee_min`, `annee_max`, `avec_performances`.

Répétable = OU (`?race=A&race=B`), dimensions différentes = ET. Insensible à la casse et aux accents. Une valeur inconnue ou un paramètre mal orthographié renvoient un **400** avec suggestion.

**Tri** : `naturel`, `nom`, `annee`, préfixés de `-` pour l'ordre décroissant. Les millésimes absents restent toujours en fin de liste.

**Pagination** : `limite` ≤ 100, `offset` ≤ 10 000. Au-delà, le fichier publié se télécharge — l'extraction en masse ne passe pas par l'API.

**Limites d'usage** : 50 **unités de quota** par seconde et par adresse, rafale de 200, puis `429` avec `Retry-After` ; concurrence dérivée du nombre de cœurs et 5 s par requête, puis `503` ; 64 paramètres et 4 Kio de chaîne de requête ; 8 générations de pedigree ; 2 000 modalités par référentiel. Toutes sont énoncées, avec leur raison d'être, dans le guide des limites.

Le quota se compte en unités parce que les requêtes ne se valent pas : une sonde pèse 50 octets, un pedigree complet 55 Kio — un facteur 4 400 pour le même prix, si l'on compte les appels. Une unité vaut 16 Kio de réponse estimée, mesurée avant exécution sur la seule URI. Presque tout coûte une unité ; seuls `vue=complet`, les référentiels entiers et les pedigrees profonds paient davantage. Un client qui restait dans le quota en boucle sur `/pedigree` tirait près d'un téraoctet par jour d'une seule adresse.

Les réponses ne portent aucun `href` : il se déduit de l'`id`, et les gabarits sont publiés par `GET /`.

## Choix techniques

### Rust

Le facteur décisif n'est pas la vitesse du langage : c'est que **le jeu tient en mémoire**. 4,4 M de fiches encodées en colonnes occupent ~1,1 Gio avec leurs index — aucune raison d'interposer une base de données. Un filtre se résout par intersection de bitmaps, un pedigree par parcours de pointeurs.

Rust apporte alors ce qui compte ici : **pas de ramasse-miettes** sur 1,1 Gio résidents, et une latence p99 prévisible. En Go, il faudrait n'utiliser que des `[]int32` sans pointeurs — c'est-à-dire écrire du Rust en Go.

Ce que Rust ne résout pas : le vrai levier de tenue en charge est le cache HTTP, pas le langage.

### REST plutôt que GraphQL

Ce jeu **contient un vrai graphe** — remonter un pedigree sur cinq générations touche 31 nœuds. L'argument « il n'y a rien à parcourir » ne tient donc pas. REST reste néanmoins le bon choix :

1. **GraphQL est hostile au cache HTTP.** Requêtes en `POST` à corps variable : ni CDN, ni proxys, ni navigateurs ne les mettent en cache. Or ce jeu est figé entre deux extractions, donc massivement cacheable.
2. **Le sur-requêtage se règle par un endpoint dédié.** `/pedigree?generations=5` répond en **une** requête cacheable, au coût borné — là où GraphQL sacrifierait le cache et ouvrirait une surface de déni de service.
3. **Interopérabilité** avec la [doctrine des API de l'État](https://guides.data.gouv.fr/guides/outils-pour-les-administrations/doctrine-des-api), qui retient REST + OpenAPI 3.1.

**Standards** : erreurs [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) (`application/problem+json`), OpenAPI 3.1, `ETag` + `Cache-Control`.

### Documentation

Deux niveaux, une seule source — `/openapi.json`, que le service publie.

**À la racine du service** : un **document de découverte** JSON — routes servies, gabarits d'URL, licence et provenance. Un client qui ne connaît que cette URL y trouve de quoi construire toutes les autres. C'est tout ce que le service publie de lui-même : la documentation destinée aux humains a quitté le binaire.

Les ancres de sa section « Erreurs » sont celles que désigne le champ `type` de chaque réponse d'erreur : un test vérifie que chaque type produit par le code y trouve sa section.

**Sur [docs.api-equides.org](https://docs.api-equides.org)** : un site [Starlight](https://starlight.astro.build) (`docs/`), avec guides, recherche et référence **générée** depuis le même `openapi.json`. Le chapeau du document OpenAPI énonce le contrat ; le raisonnement derrière chaque plafond vit dans les guides, ce qui garde la référence lisible. Là encore, aucune ressource tierce n'est chargée.

Le site se déploie séparément du service. La garantie de non-divergence n'y est plus structurelle mais procédurale : elle tient tant que le build de la documentation est rejoué après un changement d'API.

## Performances

Mesuré sur Mac 12 cœurs, `wrk -t4 -c50 -d30s`, jeu réel. ⚠️ Injecteur et serveur sur la même machine.

| Scénario | Débit | p50 | p99 |
|---|---:|---:|---:|
| Page par défaut | ≥ 125 000 req/s | 0,33 ms | 0,63 ms |
| `/healthz` (témoin, aucun travail) | 127 000 req/s | 0,34 ms | 0,49 ms |
| Répartition par race (11 747 modalités) | — | 10 ms | — |

**Les débits sont des planchers** : la vraie requête atteint le même chiffre que `/healthz`, qui ne fait rien. À ce stade, le travail applicatif ne se distingue plus du bruit du banc.

Deux optimisations ont compté. Le **tri global** est passé de 350 à 45 700 req/s grâce à des permutations pré-triées au démarrage, l'exécution choisissant la stratégie la moins chère entre parcours de permutation et tas borné. La **charge utile des listes** a été divisée par huit en servant un résumé plutôt que la fiche entière — `vue=complet` rend l'ancien comportement.

## Ce qu'il faut savoir sur la donnée

Cette API est un **miroir** : elle ne corrige rien silencieusement, elle documente. Ces points figurent aussi dans `/v1/meta`.

1. **Le champ `race` d'une fiche est pollué** : 11 747 valeurs, dont des chronos de course (`1'10"2 (ASC6H)`). La race portée par un **lien de filiation** n'en prend que 198, toutes propres (`TF`, `PS`…) — c'est celle qu'exposent `pere`, `mere` et `pere_de_mere`.
2. **568 liens de filiation sont rompus** : le parent est référencé, sa fiche absente. Ils restent exposés avec le nom connu, sans `id`, et `fiche_disponible: false`.
3. **`pere_de_mere` est redondant à 100 %** avec `mere.pere` (287 932 cas comparables, zéro divergence). Conservé par fidélité.
4. **« Sans performances » ≠ « n'a jamais concouru »** : seuls 24,9 % portent des indices publiés.
5. **Millésimes aberrants conservés** : de l'an 193 à 2026 (poulains à naître).

## Provenance et licence

Les données proviennent des fiches publiques diffusées par l'IFCE sur `infochevaux.ifce.fr`, librement accessibles sans authentification.

**L'IFCE est un établissement public administratif** ([décret n° 2010-90](https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000021725974)) :

| Texte | Portée |
|---|---|
| [CRPA L321-1](https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000031366350/LEGISCTA000032255212/) | Les informations publiques peuvent être réutilisées par toute personne |
| [CRPA L321-3](https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000033205561) | Une administration **ne peut pas opposer** le droit *sui generis* du producteur de base de données |
| [Décret 2016-1617](https://www.legifrance.gouv.fr/eli/decret/2016/11/29/PRMJ1630605D/jo/texte) | La liste des informations soumises à redevance **ne mentionne pas** les données équines |

En contrepartie (CRPA L322-1) : mentionner la source et la date de mise à jour, ne pas dénaturer. `/v1/meta` s'en charge.

**Aucune donnée à caractère personnel.** Seul champ nominatif de la source, `naisseur` est écarté dès l'ingestion — il n'est pas désérialisé et n'entre pas dans l'image. L'IFCE opère la même exclusion sur ses webservices. Un test verrouille la propriété sur les fiches, les listes, les référentiels et les filtres.

Cette analyse **n'a pas valeur d'avis juridique** ; la [CADA](https://www.cada.fr/) peut être saisie gratuitement avant une mise en ligne publique.

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

**La filiation est résolue à l'ingestion.** Le graphe étant fermé à 99,99 %, chaque identifiant de parent devient un indice de ligne : une ascendance sur cinq générations coûte 31 lectures de tableau. L'index des descendants l'inverse au chargement, en disposition CSR.

**Les index ne sont pas sérialisés.** Les reconstruire coûte 3,8 s au démarrage mais supprime toute question de compatibilité de format, et permet d'en ajouter sans réingérer.

## Déploiement

Le service tourne sur [Clever Cloud](https://clever.cloud/), sur un **runtime Rust** — pas de conteneur. `clever deploy` pousse le dépôt, la plateforme compile en `--release` et lance le binaire.

Le déploiement est automatique : une poussée sur `main` passe la CI, `semantic-release` numérote la version et pose un tag, et le tag part en production. `deploy.yml` accepte aussi un déclenchement manuel sur un tag ou un commit précis, pour revenir en arrière.

Deux secrets GitHub : `CLEVER_TOKEN` et `CLEVER_SECRET`. Côté application :

| Variable | Rôle |
|---|---|
| `CC_RUST_BIN` = `equides-api` | le dépôt produit deux binaires ; celui-ci est le serveur |
| `CC_PRE_RUN_HOOK` | télécharge `equides.bin.zst` depuis Cellar et le décompresse dans `$APP_HOME` |
| `EQUIDES_IMAGE` | l'image décompressée, dans `$APP_HOME` |
| `EQUIDES_ADRESSE` = `0.0.0.0:8080` | Clever Cloud attend l'application sur ce port |
| `EQUIDES_ADRESSE_ADMIN` = `127.0.0.1:9091` | `/metrics`, hors du port public |
| `CELLAR_ADDON_*` | fournies par l'add-on, non modifiées à la main |

**Les données ne sont pas dans le dépôt.** `make donnees` produit l'image ; compressée en `zstd`, elle est déposée sur l'add-on **Cellar (S3)**, d'où chaque instance la tire à son démarrage. Mettre le jeu à jour se réduit donc à remplacer l'objet et à redémarrer — aucun déploiement de code.

Corollaire à garder en tête : ce téléchargement conditionne le temps de démarrage, donc la réactivité de l'autoscaling.

**Mettez un CDN devant.** Avec `max-age=3600` sur un jeu figé, l'essentiel du trafic n'atteint jamais le service.

Le service est **sans état** : autant de répliques que nécessaire, ~1,1 Gio chacune, aucune coordination.

| Port | Contenu | Exposition |
|---|---|---|
| `EQUIDES_ADRESSE` (`0.0.0.0:8081`) | API publique, sondes | Internet |
| `EQUIDES_ADRESSE_ADMIN` (`127.0.0.1:9091`) | `/metrics` | **privée** |

La construction des index prend ~4 s : laissez-en autant à la sonde de démarrage avant de considérer l'instance défaillante.

**Limite connue** : ni page de statut, ni politique de version publiée — recommandations 8, 9 et 12 de la doctrine des API de l'État.

## Développement

```sh
make verifier   # fmt + clippy + tests, ce que la CI exécute
make audit      # cargo audit + versions des dépendances
make bench      # banc de charge local
```

**111 tests.** Deux méritent d'être signalés, ayant chacun trouvé un vrai défaut : `les_deux_strategies_de_tri_donnent_le_meme_resultat` compare les deux chemins du tri à une implémentation naïve, et `les_parametres_documentes_sont_exactement_ceux_acceptes` ferme l'écart entre l'OpenAPI écrit à la main et le code.

Une collection [Bruno](https://www.usebruno.com/) est versionnée dans [`bruno/`](bruno/) — 22 requêtes, 66 assertions, à lancer contre une instance chargée avec le jeu réel.

Voir [CONTRIBUTING.md](CONTRIBUTING.md).
