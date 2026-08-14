# Contribuer

```sh
make donnees    # ingère l'extraction JSONL
make verifier   # fmt + clippy + tests, ce que la CI exécute
```

## Attentes

- **`make verifier` doit passer.** La CI refuse tout avertissement clippy.
- **Toute correction de bug s'accompagne d'un test qui échoue sans elle.**
- **Code et commentaires en français**, comme la donnée. Les commentaires expliquent *pourquoi*, pas *quoi*.
- **N'altérez pas la donnée source.** Cette API est un miroir : une valeur aberrante se documente dans `/v1/meta`, elle ne se corrige pas en douce.
- **N'introduisez aucune donnée à caractère personnel.** Le champ `naisseur` a été retiré pour cette raison ; un test verrouille la propriété.

## Configuration de clippy

`pedantic` est activé dans `[lints.clippy]` du `Cargo.toml`, moins quelques familles écartées avec leur justification en commentaire.

Deux méritent attention :

- **les conversions numériques** sont écartées parce que les sites concernés sont bornés par le format d'image — mais leur audit manuel a révélé un vrai défaut, le plafond de `generations` contourné par troncature. Passez `make pedantic` de temps en temps ;
- **`many_single_char_names` et `unreadable_literal`** ne le sont que pour `date.rs`, qui reprend l'algorithme de Hinnant avec ses noms et constantes d'origine.

La version minimale de Rust est fixée par `roaring`, qui exige 1.90. Un job de CI compile avec cette exacte toolchain.

## Si vous touchez au moteur de requête

`store/` et `query.rs` portent deux chemins d'exécution pour le tri. Toute stratégie alternative doit être **comparée à une implémentation de référence naïve** sur toutes les combinaisons de paramètres : c'est ce test qui a révélé que l'inversion de la permutation inversait aussi le départage des ex æquo.

Lancez `make bench` avant et après, et joignez les deux relevés. Rappel : injecteur et serveur sur la même machine mesurent surtout l'injecteur — comparez au débit de `/healthz` que le script fournit.

## Si vous touchez à l'API

Mettez à jour la collection [`bruno/`](bruno/) et lancez-la contre une instance chargée avec le jeu réel :

```sh
cd bruno && npx @usebruno/cli run -r --env Local
```

Ses assertions portent sur la donnée réelle, elle n'est donc pas exécutée en CI.

Le document OpenAPI est écrit à la main ; un test vérifie qu'il décrit exactement les paramètres acceptés. Il a déjà signalé trois dérives.

## Mettre à jour le jeu de données

```sh
cargo run --release --bin equides-ingest -- --source nouveau.jsonl --sortie data/equides.bin
```

Vérifiez le rapport d'anomalies affiché en fin d'ingestion : doublons, dates invalides, liens rompus, amplitude des millésimes.

## Signaler un problème

Précisez l'`empreinte_source` renvoyée par `/v1/meta` et l'URL complète appelée.
