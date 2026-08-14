---
title: Erreurs
description: >-
  Format RFC 9457, catalogue des types de problèmes et conduite à tenir pour
  chacun.
sidebar:
  order: 3
---

Toutes les erreurs suivent la [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457)
(« Problem Details for HTTP APIs ») et sont servies en
`application/problem+json`. C'est ce type de média — et non le seul code de
statut — qui permet à un client générique de reconnaître un corps d'erreur.

```json
{
  "type": "https://docs.api-equides.org/guides/erreurs/#erreur-parametre-invalide",
  "title": "Paramètre invalide",
  "status": 400,
  "detail": "paramètre inconnu `rase`",
  "parametre": "rase",
  "indice": "vouliez-vous dire `race` ? paramètres admis : race, robe, sexe, …",
  "documentation": "/"
}
```

| Champ | Rôle |
|---|---|
| `type` | URI du type de problème, stable et déréférençable |
| `title` | résumé lisible, stable pour un `type` donné |
| `status` | code HTTP, répété dans le corps |
| `detail` | explication propre à cette occurrence |
| `parametre` | extension : le paramètre fautif, quand l'erreur en désigne un |
| `indice` | extension : la piste de correction |

`parametre` et `indice` sont des **extensions**, que la RFC autorise
explicitement. Ce sont elles qui rendent une erreur actionnable sans consulter
la documentation.

## Catalogue

<a id="erreur-parametre-invalide"></a>

### 400 — Paramètre invalide

`https://docs.api-equides.org/guides/erreurs/#erreur-parametre-invalide`

Un paramètre est inconnu, mal typé, hors bornes, ou porte une valeur que le
référentiel n'admet pas. `parametre` le nomme, `indice` propose la correction —
une faute de frappe reçoit la suggestion du paramètre le plus proche.

**Conduite à tenir** : corriger l'appel. Cette erreur ne se réessaie pas.

<a id="erreur-requete-invalide"></a>

### 400 — Requête invalide

`https://docs.api-equides.org/guides/erreurs/#erreur-requete-invalide`

La requête ne peut pas être traitée telle quelle, sans qu'un paramètre précis
soit en cause : chaîne de requête trop longue, trop de paramètres, appelant non
identifiable.

<a id="erreur-introuvable"></a>

### 404 — Ressource introuvable

`https://docs.api-equides.org/guides/erreurs/#erreur-introuvable`

L'identifiant ne correspond à aucune fiche, ou la route n'existe pas. Un
identifiant **mal formé** donne le même résultat qu'un identifiant inconnu :
l'API ne distingue pas les deux cas, qui appellent la même correction.

<a id="erreur-trop-de-requetes"></a>

### 429 — Trop de requêtes

`https://docs.api-equides.org/guides/erreurs/#erreur-trop-de-requetes`

Le quota de l'adresse appelante est dépassé. Le corps rappelle le coût de la
requête refusée.

**Conduite à tenir** : respecter le `Retry-After`, puis réessayer. Si les `429`
sont fréquents, la cause est souvent une requête coûteuse appelée en boucle —
voir [Limites d'usage](/guides/limites/) — ou un cache non exploité, voir
[Cache et revalidation](/guides/cache/).

<a id="erreur-service-surcharge"></a>

### 503 — Service momentanément surchargé

`https://docs.api-equides.org/guides/erreurs/#erreur-service-surcharge`

La capacité de traitement simultané est atteinte, la requête a dépassé le délai
maximal, ou aucune donnée n'est chargée.

**Conduite à tenir** : respecter le `Retry-After`. Le service refuse tôt et
clairement plutôt que de faire attendre une réponse qui expirerait ; un nouvel
essai a de bonnes chances d'aboutir.

## Réessayer proprement

Seuls les `429` et `503` se réessaient — ils portent tous deux un `Retry-After`
qui dit **quand** revenir. Les `400` et `404` désignent une erreur d'appel : les
rejouer à l'identique produira la même réponse.
