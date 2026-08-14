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

## Le champ `race` est pollué

Le champ `race` d'une fiche compte **11 747 valeurs distinctes**. Une part
d'entre elles ne sont pas des races mais des chronos de course, du genre
`1'10"2 (ASC6H)`.

La race déclarée sur un **lien de filiation**, elle, n'en prend que 13, toutes
propres. C'est celle que porte le champ `race` des objets `pere`, `mere` et
`pere_de_mere`.

Deux référentiels distincts en découlent :

```sh
curl 'https://api-equides.org/v1/referentiels/races'       # 11 747, pollué
curl 'https://api-equides.org/v1/referentiels/races_lien'  # 13, propre
```

:::tip
Pour classer ou facetter proprement, préférez `races_lien`. Pour filtrer sur ce
que porte réellement une fiche, il faut `race`.
:::

## « Sans performances » ne veut pas dire « n'a jamais couru »

Le filtre `avec_performances=false` ne désigne pas les équidés qui n'ont jamais
concouru, mais ceux pour lesquels **aucun indice n'est publié**. C'est le cas de
75,1 % du jeu : seuls 24,9 % des équidés portent un indice.

L'absence d'indice est une absence de publication, pas une absence de carrière.

## Liens de filiation rompus

Sur 1 202 446 parents référencés, 1 202 286 figurent au jeu de données. Les 160
restants sont des **liens rompus** : un parent est nommé, mais sa fiche est
absente.

Ces liens sont exposés comme une **absence de parent**, jamais comme une
référence morte qui renverrait un 404. Le champ `fiche_disponible` d'un objet
`Reference` vaut alors `false`.

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

## Le champ `url`

Il n'est pas stocké mais **reconstruit** depuis le slug et l'identifiant, selon
le gabarit qu'expose `url_modele` dans `/v1/meta`. Il pointe vers la fiche
d'origine, qui fait autorité.

## Dénombrement

`/v1/meta` expose sous `anomalies` le décompte exact de chacun de ces défauts,
établi à l'ingestion : millésimes absents, robes absentes, fiches sans
filiation, parents pendants, identifiants illisibles et slugs non déductibles.
