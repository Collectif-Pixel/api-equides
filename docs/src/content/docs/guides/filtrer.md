---
title: Filtrer et paginer
description: >-
  Combiner les critères, découvrir les valeurs admises, trier et découper une
  sélection.
sidebar:
  order: 2
---

## Combiner les critères

Les valeurs d'une **même** dimension se combinent en OU, les dimensions
**différentes** en ET.

```sh
# Trotteurs OU Pur Sang, et femelles, et nés entre 2010 et 2015
curl 'https://api-equides.org/v1/equides?race=Trotteur%20Francais&race=Pur%20Sang&sexe=Femelle&annee_min=2010&annee_max=2015'
```

Deux écritures sont acceptées indifféremment pour les valeurs multiples :

```sh
curl '…/v1/equides?race=Trotteur%20Francais&race=Pur%20Sang'   # répétition
curl '…/v1/equides?race=Trotteur%20Francais,Pur%20Sang'        # virgules
```

Les valeurs sont insensibles à la casse et aux accents : `pur sang`,
`PUR SANG` et `Pur Sang` désignent la même race.

## Découvrir les valeurs admises

Sans cette liste, rien ne laisse deviner que la source écrit `Trotteur Francais`
sans cédille ni accent, ni que le sexe se dit `Femelle` et non `F`.

```sh
curl 'https://api-equides.org/v1/referentiels/races'
```

| Dimension | Contenu |
|---|---|
| `races` | le champ `race` d'une fiche — [pollué par la source](/donnees/defauts/) |
| `races_lien` | les 13 races propres déclarées sur les liens de filiation |
| `robes` | robes renseignées |
| `sexes` | `Femelle`, `Male`, `Hongre`, `Indeter` |
| `disciplines` | disciplines portant des performances |
| `codes_indice` | `BTR`, `ISO`, `ITR`… |
| `annees_naissance` | millésimes, par ordre chronologique |

Les listes sont rendues de la modalité la plus fréquente à la plus rare, et
tronquées à 2 000 entrées : `nombre_de_modalites` donne alors le total réel et
`note` signale la troncature.

## Trier

`tri` accepte `naturel` (ordre du fichier source, défaut), `nom` ou `annee`.
Le préfixe `-` inverse le sens.

```sh
curl 'https://api-equides.org/v1/equides?tri=-annee'
```

Les millésimes absents sont **toujours** placés en fin de liste, quel que soit
le sens : un tri décroissant ne doit pas commencer par ce qu'on ne sait pas.

## Choisir le détail

| `vue` | Contenu | Poids |
|---|---|---|
| `resume` (défaut) | id, nom, race, sexe, robe, millésime | ~139 o par ligne |
| `complet` | la fiche entière, performances comprises | ~795 o par ligne |

Servir des résumés par défaut allège une page de cent lignes d'un facteur six.
La fiche complète se demande explicitement, ou se récupère une par une.

:::note
`vue=complet` coûte davantage de quota, à proportion de ce qu'il fait produire.
Voir [Limites d'usage](/guides/limites/).
:::

## Paginer

`limite` vaut 100 au maximum (20 par défaut), `offset` 10 000.

```sh
curl 'https://api-equides.org/v1/equides?race=Pur%20Sang&limite=100&offset=200'
```

La réponse porte les liens des pages voisines, filtres conservés :

```json
{
  "donnees": [ "…" ],
  "pagination": {
    "total": 128034,
    "limite": 100,
    "offset": 200,
    "suivant": "/v1/equides?race=Pur+Sang&limite=100&offset=300",
    "precedent": "/v1/equides?race=Pur+Sang&limite=100&offset=100"
  }
}
```

:::caution[L'extraction en masse ne passe pas par l'API]
Au-delà de 10 000 de profondeur, la pagination s'arrête. Affinez les filtres,
ou téléchargez le fichier publié — `/v1/meta` en donne la source et la date.
:::

## Compter sans rapatrier

Pour peupler des facettes, `/v1/stats/repartition` ventile la **sélection
filtrée** selon une dimension. Seuls les effectifs traversent le réseau.

```sh
curl 'https://api-equides.org/v1/stats/repartition?sexe=Femelle&dimension=race&limite=10'
```

```json
{
  "dimension": "race",
  "effectif_filtre": 1908445,
  "effectif_affiche": 1755302,
  "donnees": [
    { "valeur": "Trotteur Francais", "nombre": 612884 },
    { "valeur": "Pur Sang", "nombre": 128034 }
  ]
}
```

`effectif_affiche` est inférieur à `effectif_filtre` dès que `limite` tronque la
liste, ou que des lignes n'ont pas de valeur pour cette dimension.
