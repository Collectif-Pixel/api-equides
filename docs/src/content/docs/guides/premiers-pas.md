---
title: Premiers pas
description: Premier appel, identifiants, formats et conventions de l'API Équidés.
sidebar:
  order: 1
---

L'API est publique et non authentifiée : ni clé, ni jeton, ni compte. Il n'y a
rien à demander avant d'appeler.

```sh
curl 'https://api-equides.org/v1/search?q=qabalah'
```

## Trouver un équidé

Deux routes cherchent par nom, et elles ne servent pas le même usage.

| Route | Pour | Ordre |
|---|---|---|
| `/v1/search` | alimenter un champ de saisie | par pertinence |
| `/v1/equides?nom=…` | filtrer une sélection | par tri demandé, paginé |

`/v1/search` classe et borne son coût pour répondre avant la frappe suivante.
`/v1/equides` filtre et pagine, sans classer.

```sh
curl 'https://api-equides.org/v1/search?q=qab&limite=5'
```

```json
{
  "q": "qab",
  "limite": 5,
  "resultats": [
    {
      "id": "Z4ogLhlkS2CeUdq0bZ0YFw",
      "nom": "QABALAH MERCURY",
      "race": "Trotteur Francais",
      "sexe": "Femelle",
      "annee_naissance": 2026,
      "score": 1
    }
  ]
}
```

## L'identifiant

Le champ `id` est **l'identifiant officiel de l'IFCE** : celui qui figure tel
quel dans l'URL de la fiche publique sur `infochevaux.ifce.fr`. Vingt-deux
caractères en base64 URL.

Il ouvre les trois routes d'une fiche :

```sh
curl 'https://api-equides.org/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw'
curl 'https://api-equides.org/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/pedigree'
curl 'https://api-equides.org/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw/descendance'
```

Aucune réponse ne répète un lien qui se déduit d'un identifiant : cela allégeait
chaque ligne de liste d'une quarantaine d'octets pour zéro information. Les
gabarits d'URL sont publiés une fois pour toutes par le document de découverte.

```sh
curl -H 'Accept: application/json' 'https://api-equides.org/'
```

## Conventions

Toutes les routes répondent en `GET` (et `HEAD`) et servent de
l'`application/json; charset=utf-8`.

Un **paramètre inconnu est refusé, pas ignoré**. Une faute de frappe ne peut
donc pas se traduire silencieusement par le jeu entier :

```sh
curl 'https://api-equides.org/v1/equides?rase=Pur%20Sang'
```

```json
{
  "type": "https://docs.api-equides.org/guides/erreurs/#erreur-parametre-invalide",
  "title": "Paramètre invalide",
  "status": 400,
  "detail": "paramètre inconnu `rase`",
  "parametre": "rase",
  "indice": "vouliez-vous dire `race` ? paramètres admis : race, robe, sexe, …"
}
```

Les erreurs suivent la RFC 9457 — voir [Erreurs](/guides/erreurs/).

## Et ensuite

- [Filtrer et paginer](/guides/filtrer/) — combiner les critères, trier, découper
- [Généalogie](/guides/genealogie/) — ascendance et produits directs
- [Cache et revalidation](/guides/cache/) — le levier de tenue en charge
- [Limites d'usage](/guides/limites/) — ce que borne le service, et pourquoi
