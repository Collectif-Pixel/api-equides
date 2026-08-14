---
title: Provenance et licence
description: >-
  Origine de la donnée servie, régime de réutilisation et mention obligatoire.
sidebar:
  order: 1
---

:::caution[Service indépendant]
API Équidés n'est **pas affiliée à l'IFCE**. La donnée servie est une extraction
datée ; les fiches en ligne sur `infochevaux.ifce.fr` font seules autorité et
peuvent diverger de ce que sert cette API.
:::

## Ce qui est servi

4 446 638 fiches publiques d'équidés enregistrés en France, extraites de
`infochevaux.ifce.fr`. `/v1/meta` en donne l'état exact :

```sh
curl 'https://api-equides.org/v1/meta'
```

| Champ | Rôle |
|---|---|
| `source` | l'extraction dont proviennent les fiches |
| `ingere_le` | la date de mise à jour, à citer dans la mention |
| `empreinte_source` | l'empreinte blake3, qui identifie l'extraction sans ambiguïté |
| `lignes` | le nombre de fiches chargées |
| `anomalies` | les défauts dénombrés à l'ingestion |

## Réutilisation

La donnée relève de la **Licence Ouverte / réutilisation d'informations
publiques**, au titre du code des relations entre le public et l'administration.

L'IFCE est un établissement public administratif (décret n° 2010-90).
L'article L321-1 du CRPA ouvre la réutilisation des informations publiques qu'il
diffuse ; l'article L321-3 lui interdit d'opposer le droit *sui generis* du
producteur de base de données ; et le décret n° 2016-1617, qui fixe
limitativement les informations soumises à redevance, ne mentionne ni l'IFCE ni
les données équines.

### Ce que la réutilisation impose

L'article L322-1 du CRPA impose de **mentionner la source et la date de mise à
jour**, et de ne pas dénaturer la donnée. Les champs `source` et `ingere_le`
de `/v1/meta` y pourvoient.

Une mention conforme ressemble à ceci :

> Source : IFCE, fiches publiques d'infochevaux.ifce.fr, extraction du
> {date de `ingere_le`}, via API Équidés (service indépendant).

## Données personnelles

Cette API **n'expose aucune donnée à caractère personnel** : elle ne sert que
des informations relatives à des animaux.

## Le code

Le service est publié sous licence MIT. La licence du **code** et celle des
**données** sont distinctes : la première ne vous dispense pas de la mention
qu'impose la seconde.
