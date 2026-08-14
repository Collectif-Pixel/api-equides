# Documentation de l'API Équidés

Site [Starlight](https://starlight.astro.build) servant la documentation de
`api-equides.org`. Les guides sont écrits à la main ; **la référence est
générée** depuis `openapi.json`, que le service publie lui-même.

## Développer

```sh
npm install
npm run dev      # http://localhost:4321
```

## Construire

```sh
npm run build            # synchronise openapi.json puis construit
npm run build:hors-ligne # construit depuis l'exemplaire versionné
```

`npm run sync` récupère `https://api-equides.org/openapi.json`. La variable
`OPENAPI_URL` permet de viser une autre instance :

```sh
OPENAPI_URL=http://127.0.0.1:8081/openapi.json npm run sync
```

Un exemplaire d'`openapi.json` est versionné pour que la construction reste
possible hors ligne, mais il n'est là qu'en repli : **le build normal
synchronise**. C'est ce qui empêche la documentation de décrire une API que le
service n'expose plus.

## Structure

| Chemin | Contenu |
|---|---|
| `src/content/docs/guides/` | prise en main, filtres, généalogie, cache, limites, erreurs |
| `src/content/docs/donnees/` | provenance, licence, défauts connus de la source |
| `src/content/docs/index.mdx` | page d'accueil |
| `/reference/` | **généré** depuis `openapi.json`, ne pas écrire à la main |

## Déployer

Application **statique** Clever Cloud, distincte du service Rust.

```sh
clever create --type static-apache api-equides-docs
clever env set CC_WEBROOT /docs/dist
clever env set CC_PRE_BUILD_HOOK "cd docs && npm ci && npm run build"
clever domain add docs.api-equides.org
clever deploy
```

Le service Rust et la documentation se déploient séparément. Rejouer le build
de la documentation après tout changement d'API est ce qui garantit que la
référence reste exacte — le service, lui, sert toujours `/openapi.json`, qui
fait foi.

## Ce que le site ne charge pas

Aucune ressource tierce : ni police distante, ni script de CDN, ni télémétrie.
Le lecteur d'une documentation publique n'a pas à laisser son adresse à un
tiers pour la consulter. La recherche est assurée par Pagefind, construite au
moment du build et servie depuis le même domaine.
