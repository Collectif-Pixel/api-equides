---
title: Généalogie
description: Remonter une ascendance et lister les produits directs d'un équidé.
sidebar:
  order: 3
---

Ce jeu contient un vrai graphe : 4,4 millions de fiches dont la filiation est
résolue à 99,99 %. Deux routes le parcourent, dans les deux sens.

## Ascendance

```sh
curl 'https://api-equides.org/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/pedigree?generations=5'
```

En REST naïf, remonter un pedigree coûterait une requête par ancêtre — 31
allers-retours sur cinq générations. La filiation étant résolue en indices de
ligne dès l'ingestion, cet endpoint répond en **une seule requête cacheable**,
au coût borné et connu à l'avance.

```json
{
  "generations": 5,
  "noeuds": 24,
  "noeuds_theoriques": 63,
  "arbre": {
    "id": "Z4ogLhlkS2CeUdq0bZ0YFw",
    "nom": "QABALAH MERCURY",
    "generation": 0,
    "pere": { "nom": "BOOSTER WINNER", "generation": 1, "pere": { "…": "…" } },
    "mere": { "nom": "JEWELLE DARK", "generation": 1 }
  }
}
```

### Mesurer les lacunes

`noeuds` compte les ancêtres **réellement connus**, `noeuds_theoriques` ceux
qu'un pedigree complet compterait. L'écart mesure les lacunes de l'ascendance :
ci-dessus, 24 ancêtres sur 63 possibles.

Une branche s'interrompt quand un parent est référencé mais absent du jeu. Ces
liens rompus sont exposés comme une **absence de parent**, jamais comme une
référence morte — voir [Défauts connus](/donnees/defauts/).

### Profondeur

`generations` vaut 4 par défaut, 8 au maximum. Un arbre double de taille à
chaque rang : huit générations représentent déjà jusqu'à 511 ancêtres, soit la
plus lourde réponse de l'API.

| Générations | Ancêtres théoriques | Réponse |
|---|---|---|
| 4 (défaut) | 31 | ~3 Kio |
| 6 | 127 | ~14 Kio |
| 8 (maximum) | 511 | ~55 Kio |

:::note
Au-delà de 8, la réponse est un `400`. La profondeur pèse aussi sur le quota :
voir [Limites d'usage](/guides/limites/).
:::

## Descendance

Réponse symétrique, servie par un index inverse bâti au chargement.

```sh
curl 'https://api-equides.org/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/descendance?limite=50'
```

```json
{
  "parent": { "id": "Z4ogLhlkS2CeUdq0bZ0YFw", "nom": "QABALAH MERCURY" },
  "total": 214,
  "limite": 50,
  "offset": 0,
  "donnees": [ { "id": "…", "nom": "…", "race": "…" } ]
}
```

Seuls les **produits directs** sont renvoyés — les fiches ayant cet équidé pour
père ou mère. La descendance complète se parcourt en suivant les identifiants
de proche en proche.

## La race d'un parent

Le champ `race` des objets `pere`, `mere` et `pere_de_mere` ne vient **pas** de
la fiche du parent : il vient de la race déclarée sur le lien de filiation. Ces
deux valeurs diffèrent, et celle du lien est la propre — 13 modalités contre
11 747. La raison est expliquée dans [Défauts connus](/donnees/defauts/).
