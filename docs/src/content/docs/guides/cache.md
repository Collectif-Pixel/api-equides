---
title: Cache et revalidation
description: >-
  ETag, Cache-Control et If-None-Match : le principal levier de tenue en charge
  de l'API Équidés.
sidebar:
  order: 1
---

Le jeu de données est **figé entre deux extractions**. C'est ce qui rend cette
API massivement cacheable, et le cache est son principal levier de tenue en
charge — bien avant le langage dans lequel elle est écrite.

## Revalider

Toutes les réponses portent un `ETag` et un `Cache-Control` d'une heure.

```sh
curl -i 'https://api-equides.org/v1/stats'
```

```http
HTTP/1.1 200 OK
ETag: "a3f9c2e18b7d4056f1a2c8e4"
Cache-Control: public, max-age=3600, stale-while-revalidate=86400
```

Renvoyez cet `ETag` en `If-None-Match` :

```sh
curl -i -H 'If-None-Match: "a3f9c2e18b7d4056f1a2c8e4"' 'https://api-equides.org/v1/stats'
```

```http
HTTP/1.1 304 Not Modified
```

Un `304` ne porte aucun corps et **ne recalcule rien** : l'empreinte est
comparée avant même d'exécuter la requête. Une revalidation coûte donc
infiniment moins cher que l'appel initial — pour vous comme pour le service.

## Ce que couvre l'ETag

L'empreinte est calculée sur trois éléments :

- l'**empreinte du jeu de données** chargé, qui change à chaque extraction ;
- l'**URI complète**, chemin et chaîne de requête ;
- l'**encodage de transfert** négocié.

Deux requêtes qui ne diffèrent que par un paramètre ne partagent donc pas de
validateur, et deux encodages différents non plus — un cache intermédiaire ne
peut pas servir du gzip à un client qui n'en veut pas.

## Compression

Les réponses sont compressées selon l'`Accept-Encoding` de la requête. Le gain
est considérable sur les réponses structurées :

```sh
curl -s -o /dev/null -w '%{size_download}\n' \
  'https://api-equides.org/v1/equides/{id}/pedigree?generations=8'
# 55 Ko

curl -s --compressed -o /dev/null -w '%{size_download}\n' \
  'https://api-equides.org/v1/equides/{id}/pedigree?generations=8'
# 8 Ko
```

Un facteur sept, pour un en-tête. Demandez la compression.

## Détecter un changement de jeu

`/v1/meta` expose l'empreinte exacte de l'extraction servie :

```sh
curl 'https://api-equides.org/v1/meta'
```

```json
{
  "jeu_de_donnees": {
    "source": "Extraction des fiches publiques d'infochevaux.ifce.fr",
    "ingere_le": "2026-08-14",
    "empreinte_source": "…",
    "lignes": 4446638
  }
}
```

`empreinte_source` identifie l'extraction sans ambiguïté. Si vous conservez des
données dérivées, c'est le champ qui dit s'il faut les recalculer — et
`ingere_le` est la date que la mention de réutilisation doit citer.
