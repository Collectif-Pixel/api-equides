---
title: Limites d'usage
description: >-
  Quota pondéré, concurrence, délai et plafonds de l'API Équidés, avec leur
  raison d'être.
sidebar:
  order: 2
---

Cette API est publique et non authentifiée. Les plafonds ci-dessous la protègent
d'une charge disproportionnée ; ils sont larges pour tout usage normal.

## Débit

**50 unités de quota par seconde et par adresse**, avec une rafale de 200.
Au-delà : `429`, assorti d'un `Retry-After` indiquant le délai à respecter.

### Pourquoi des unités

Les requêtes ne se valent pas. Une sonde pèse 50 octets, un pedigree complet
55 Kio — un facteur 1 100. Compter les appels reviendrait à laisser le plus
cher des endpoints définir la charge que le service doit encaisser.

Une unité vaut **16 Kio de réponse estimée**, mesurée sur la seule URI avant
d'exécuter quoi que ce soit. Presque tout coûte donc une unité :

| Requête | Coût |
|---|---|
| Une fiche, une sonde, `/v1/meta`, `/v1/stats` | 1 |
| Une page de 100 résumés | 1 |
| Une autocomplétion, une page de descendance | 1 |
| Une répartition sur 1 000 modalités | 3 |
| Un pedigree à 8 générations | 4 |
| 100 lignes en `vue=complet` | 5 |
| Un référentiel entier | 5 |

L'usage courant ne paie donc rien de plus qu'un décompte par appel. Le corps du
`429` rappelle le coût de la requête refusée.

:::tip
Une revalidation `If-None-Match` qui renvoie `304` est ce qui coûte le moins.
Voir [Cache et revalidation](/guides/cache/).
:::

## Concurrence et délai

**8 requêtes traitées simultanément**, valeur dérivée du nombre de cœurs de
l'instance. Au-delà : `503` avec `Retry-After`.

Le travail servi ici tient en mémoire et sature le processeur — intersections de
bitmaps, sérialisation. Au-delà de quelques requêtes par cœur, on ne gagne pas
de parallélisme, on forme une file d'attente derrière laquelle chacun attend
d'autant plus longtemps. Le service refuse tôt plutôt que de faire attendre une
réponse qui expirerait.

**5 secondes par requête**, au-delà desquelles le service abandonne et répond
`503`. Aucune requête servie n'en approche : la plus lourde se mesure en
dizaines de millisecondes.

## Plafonds de paramètres

| Limite | Valeur | Dépassement |
|---|---|---|
| Lignes par page | 100 | `400` sur `/v1/equides` |
| Profondeur de pagination | 10 000 | `400` |
| Générations de pedigree | 8 | `400` |
| Modalités d'une répartition | 1 000 | ramené au plafond |
| Suggestions d'autocomplétion | 100 | ramené au plafond |
| Modalités d'un référentiel | 2 000 | tronqué, `note` le signale |
| Paramètres par requête | 64 | `400` |
| Chaîne de requête | 4 096 octets | `400` |

Le dépassement de `limite` ne se traite pas partout de la même façon.
`/v1/equides` **refuse** une valeur hors bornes ; `/v1/search`,
`/v1/equides/{id}/descendance` et `/v1/stats/repartition` la **ramènent** au
plafond. Une autocomplétion ne doit pas échouer sur un réglage trop généreux.

Le plafond de paramètres n'est pas décoratif : chaque valeur répétée coûte une
résolution de dictionnaire et une union de bitmaps. Sans lui, une seule URL
pourrait demander un travail sans commune mesure avec sa taille.

## Identification de l'appelant

Le quota s'impute à une adresse. Derrière un relais, l'appelant est identifié
par `X-Forwarded-For` **s'il provient d'un relais déclaré** ; un client direct
est identifié par son adresse, et l'en-tête est alors ignoré.

C'est délibéré : `X-Forwarded-For` est un en-tête que le client contrôle. En
prendre naïvement la première valeur permettrait d'en forger une nouvelle à
chaque requête pour disposer d'un quota neuf, et la limitation ne bornerait plus
personne.

## Extraction en masse

Elle ne passe pas par l'API. Le jeu complet se télécharge : `/v1/meta` en donne
la source, la date et l'empreinte. Paginer 4,4 millions de lignes serait plus
lent pour vous et plus coûteux pour tout le monde.
