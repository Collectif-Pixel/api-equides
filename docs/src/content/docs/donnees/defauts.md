---
title: Défauts connus
description: >-
  Ce que la source contient de bancal, et pourquoi l'API le conserve tel quel.
sidebar:
  order: 2
---

Cette API est un **miroir fidèle** de la source, pas un correcteur. Les défauts
ci-dessous viennent de la donnée d'origine et ne sont pas retouchés : les
signaler vaut mieux que les masquer, car un correctif silencieux produirait des
chiffres qui ne se raccordent à rien.

## Deux nomenclatures de race

Ce n'est pas un défaut, mais le piège d'intégration le plus fréquent. La source
emploie deux vocabulaires de race qui **ne se recouvrent pas** :

| Où | Forme | Référentiel |
|---|---|---|
| champ `race` d'une fiche | libellé développé — `Trotteur Francais` | `races` |
| champ `race` d'un `pere`, `mere`, `pere_de_mere` | code abrégé du lien — `TF` | `races_lien` |

```sh
curl 'https://api-equides.org/v1/referentiels/races'
curl 'https://api-equides.org/v1/referentiels/races_lien'
```

:::caution
Le filtre `race=` n'accepte que la **première** forme. Passer un code de lien
(`race=TF`) renvoie un `400`, avec le référentiel à consulter en indice.
:::

Les effectifs de chaque nomenclature sont donnés par `nombre_de_modalites` sur
ces deux routes, et repris par `/v1/meta` : ils ne sont pas recopiés ici, pour
qu'aucune page ne puisse annoncer un chiffre que l'API contredit.

## « Sans performances » ne veut pas dire « n'a jamais couru »

Le filtre `avec_performances=false` ne désigne pas les équidés qui n'ont jamais
concouru, mais ceux pour lesquels **aucun indice n'est publié**.

L'absence d'indice est une absence de publication, pas une absence de carrière.
`/v1/stats` donne la part réelle via `avec_performances` et `total`.

## Liens de filiation rompus

Un **lien rompu** est un parent nommé dont la fiche est absente du jeu. Ils sont
exposés comme une **absence de parent**, jamais comme une référence morte qui
renverrait un 404 ; le champ `fiche_disponible` d'un objet `Reference` vaut
alors `false`.

`/v1/meta` en donne deux dénombrements qu'il ne faut pas confondre :

| Champ d'`anomalies` | Ce qu'il compte |
|---|---|
| `parents_references` | identifiants de parents **distincts** cités |
| `parents_pendants` | ceux d'entre eux qui manquent au jeu |
| `references_de_parents` | emplacements `pere`, `mere`, `pere_de_mere` renseignés |
| `references_pendantes` | ceux d'entre eux dont la cible manque |

`references_pendantes` est toujours supérieur ou égal à `parents_pendants` : un
même étalon absent est cité par toutes ses fiches de produits. Les deux mesurent
le même défaut, l'un par citation, l'autre par identifiant.

C'est ce qui explique l'écart entre `noeuds` et `noeuds_theoriques` d'un
pedigree — voir [Généalogie](/guides/genealogie/).

## Millésimes aberrants

La source contient des millésimes de naissance impossibles : l'extraction
descend jusqu'à l'an 193, et comporte des poulains à naître dont le millésime
est postérieur à la date d'extraction.

Ils sont **conservés tels quels**. `/v1/stats` en expose les bornes réelles via
`annee_naissance_min` et `annee_naissance_max` : filtrez sur `annee_min` et
`annee_max` si votre usage ne les tolère pas.

## Champs absents

Un champ absent est **omis**, jamais rendu à `null`. Une fiche sans robe n'a pas
de clé `robe` ; un équidé sans millésime n'a pas de clé `annee_naissance`.

Cela allège les réponses, et distingue sans ambiguïté « non renseigné » de
« renseigné à zéro ».

## Déclaré n'est pas calculé

Deux champs de fiche sont **déclarés par la source** et ne se reconstruisent pas
par le calcul :

- `record`, le chrono de course, sous la forme `1'16"7 (AEL3V)`. C'est du texte :
  ni comparable, ni triable en l'état.
- `statut_reproducteur` — `Pouliniere`, `Etalon Actif`… — que
  `nombre_de_descendants`, lui, est calculé par cette API.

Les deux derniers ne font pas double emploi. Une jument déclarée `Pouliniere`
sans aucun produit enregistré n'est pas une jument jamais mise à la
reproduction ; c'est une distinction que la descendance seule ne rend pas.

:::note
L'absence de `statut_reproducteur` est un **silence de la source**, pas une
négation.
:::

## Le champ `url`

Il n'est pas stocké mais **reconstruit** depuis le slug et l'identifiant, selon
le gabarit qu'expose `url_modele` dans `/v1/meta`. Il pointe vers la fiche
d'origine, qui fait autorité.

## Dénombrement

`/v1/meta` expose sous `anomalies` le décompte exact de chacun de ces défauts,
établi à l'ingestion : millésimes absents, robes absentes, fiches sans
filiation, parents et citations pendants, identifiants illisibles et slugs non
déductibles.

Les `remarques` du même document sont **dérivées du jeu chargé** à chaque
requête, jamais recopiées à la main : les chiffres qu'elles citent sont ceux que
les autres routes servent, par construction.
