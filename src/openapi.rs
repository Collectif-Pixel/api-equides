use crate::api::equides::{DESCENDANCE_DEFAUT, GENERATIONS_DEFAUT, SUGGESTIONS_DEFAUT};
use crate::api::erreur;
use crate::api::referentiels::MODALITES_MAX;
use crate::api::stats::{MODALITES_REPARTITION_DEFAUT, MODALITES_REPARTITION_MAX};
use crate::query::{GENERATIONS_MAX, LIMITE_DEFAUT, LIMITE_MAX, OFFSET_MAX};
use crate::store::Store;
use serde_json::{Value, json};

fn plus_frequent<'a>(
    dict: &'a crate::store::Dict,
    bitmap: impl Fn(u32) -> Option<&'a roaring::RoaringBitmap>,
) -> String {
    dict.iter()
        .max_by_key(|(code, _)| bitmap(*code).map_or(0, roaring::RoaringBitmap::len))
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

fn exemples(store: &Store) -> (String, String, String) {
    (
        plus_frequent(&store.races, |c| store.bitmap_race(c)),
        plus_frequent(&store.robes, |c| store.bitmap_robe(c)),
        plus_frequent(&store.sexes, |c| store.bitmap_sexe(c)),
    )
}

fn param(nom: &str, description: &str, schema: &Value, exemple: &Value) -> Value {
    json!({
        "name": nom,
        "in": "query",
        "required": false,
        "description": description,
        "schema": schema,
        "example": exemple,
    })
}

fn part_sans_performances(store: &Store) -> String {
    let total = u64::from(store.n);
    if total == 0 {
        return "0,0".to_string();
    }
    let sans = total - store.bitmap_avec_performances().len();
    format!("{:.1}", sans as f64 * 100.0 / total as f64).replace('.', ",")
}

fn params_filtre(store: &Store) -> Vec<Value> {
    let (race, robe, sexe) = exemples(store);
    let repetable = |t: &str| json!({ "type": "array", "items": { "type": t } });
    let statut = plus_frequent(&store.statuts_reproducteur, |c| {
        store.bitmap_statut_reproducteur(c)
    });
    vec![
        param(
            "race",
            &format!(
                "Race, répétable pour un OU (`?race=A&race=B`) ou séparée par des virgules. \
                 Insensible à la casse et aux accents. Le jeu compte {} races distinctes ; \
                 valeurs admises sur /v1/referentiels/races. Attention : les objets `pere`, \
                 `mere` et `pere_de_mere` portent le code abrégé du lien \
                 (/v1/referentiels/races_lien), que ce filtre n'accepte pas.",
                store.races.len()
            ),
            &repetable("string"),
            &json!(race),
        ),
        param(
            "robe",
            "Robe, mêmes règles que `race`. Valeurs admises : /v1/referentiels/robes.",
            &repetable("string"),
            &json!(robe),
        ),
        param(
            "sexe",
            "Sexe tel qu'écrit par la source : `Femelle`, `Male`, `Hongre`, `Indeter`.",
            &repetable("string"),
            &json!(sexe),
        ),
        param(
            "statut_reproducteur",
            "Statut déclaré à la reproduction. Distinct de la descendance connue : une \
             poulinière sans produit enregistré porte quand même son statut. \
             Valeurs admises : /v1/referentiels/statuts_reproducteur.",
            &repetable("string"),
            &json!(statut),
        ),
        param(
            "discipline",
            "Restreint aux équidés portant des performances dans cette discipline. \
             Valeurs admises : /v1/referentiels/disciplines.",
            &repetable("string"),
            &json!("TROT COURSE"),
        ),
        param(
            "indice",
            "Restreint aux équidés portant un indice de ce code (`BTR`, `ISO`, `ITR`…). \
             Valeurs admises : /v1/referentiels/codes_indice.",
            &repetable("string"),
            &json!("ISO"),
        ),
        param(
            "annee_naissance",
            "Millésime de naissance, répétable.",
            &repetable("integer"),
            &json!(2015),
        ),
        param(
            "nom",
            "Recherche sur le nom. Chaque mot de la requête est traité comme un préfixe et \
             tous doivent être présents : `qabalah mer` retrouve « QABALAH MERCURY ».",
            &json!({ "type": "string" }),
            &json!("qabalah"),
        ),
        param(
            "annee_min",
            "Millésime de naissance minimal, inclus.",
            &json!({ "type": "integer" }),
            &json!(2010),
        ),
        param(
            "annee_max",
            "Millésime de naissance maximal, inclus.",
            &json!({ "type": "integer" }),
            &json!(2020),
        ),
        param(
            "avec_performances",
            &format!(
                "Restreint aux équidés portant (ou non) des indices de performance. \
                 Accepte `true`, `false`, `1`, `0`, `oui`, `non`. \
                 Attention : `false` ne signifie pas que l'équidé n'a jamais concouru, \
                 seulement qu'aucun indice n'est publié — {} % du jeu est dans ce cas.",
                part_sans_performances(store)
            ),
            &json!({ "type": "boolean" }),
            &json!(true),
        ),
    ]
}

fn schemas() -> Value {
    json!({
        "Reference": {
            "type": "object",
            "description": "Renvoi vers un autre équidé du jeu de données.",
            "properties": {
                "id": { "type": "string", "example": "gecpqPj6Rc2vy_Hz_Y1DmQ" },
                "nom": { "type": "string", "example": "BOOSTER WINNER" },
                "race": { "type": "string" },
                "fiche_disponible": {
                    "type": "boolean",
                    "description": "Faux si la fiche du parent est absente du jeu de données."
                }
            }
        },
        "Equide": {
            "type": "object",
            "required": ["id", "nom", "slug", "url", "race", "sexe", "nombre_de_descendants"],
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Identifiant officiel IFCE, celui qui figure dans l'URL de \
                                    la fiche publique. 22 caractères en base64 URL.",
                    "example": "Z4ogLhlkS2CeUdq0bZ0YFw"
                },
                "nom": { "type": "string", "example": "QABALAH MERCURY" },
                "slug": { "type": "string", "example": "qabalah-mercury" },
                "url": {
                    "type": "string",
                    "description": "Fiche d'origine, reconstruite depuis le slug et l'identifiant."
                },
                "race": {
                    "type": "string",
                    "description": "Libellé développé. Les objets `pere`, `mere` et \
                                    `pere_de_mere` portent, eux, le code abrégé du lien.",
                    "example": "Trotteur Francais"
                },
                "sexe": { "type": "string", "enum": ["Femelle", "Male", "Hongre", "Indeter"] },
                "robe": { "type": "string", "example": "Bai" },
                "annee_naissance": { "type": "integer", "example": 2026 },
                "record": {
                    "type": "string",
                    "description": "Chrono de course déclaré par la source. Texte : ni \
                                    comparable ni triable en l'état. Absent quand la source \
                                    n'en publie pas.",
                    "example": "1'16\"7 (AEL3V)"
                },
                "statut_reproducteur": {
                    "type": "string",
                    "description": "Statut déclaré à la reproduction, là où \
                                    `nombre_de_descendants` est calculé : une poulinière sans \
                                    produit enregistré porte quand même son statut. Son absence \
                                    est un silence de la source, pas une négation.",
                    "example": "Pouliniere"
                },
                "filiation_texte": {
                    "type": "string",
                    "description": "Filiation telle que rédigée par la source.",
                    "example": "Par BOOSTER WINNER TF et JEWELLE DARK TF par CHARLY DU NOYER TF"
                },
                "pere": { "$ref": "#/components/schemas/Reference" },
                "mere": { "$ref": "#/components/schemas/Reference" },
                "pere_de_mere": { "$ref": "#/components/schemas/Reference" },
                "nombre_de_descendants": {
                    "type": "integer",
                    "description": "Fiches ayant cet équidé pour père ou mère."
                },
                "performances": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Performance" }
                },
            }
        },
        "Performance": {
            "type": "object",
            "properties": {
                "discipline": { "type": "string", "example": "TROT COURSE" },
                "texte": { "type": "string", "example": "BTR +59 (0.35)" },
                "indices": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "code": { "type": "string", "example": "BTR" },
                            "valeur": { "type": "string", "example": "+59" },
                            "coefficient": { "type": "number", "example": 0.35 },
                            "annee": { "type": "integer" },
                            "appreciation": { "type": "string", "example": "indice-satisfaisant" }
                        }
                    }
                }
            }
        },
        "Noeud": {
            "type": "object",
            "description": "Nœud d'un arbre d'ascendance ; `pere` et `mere` sont des Noeud.",
            "properties": {
                "id": { "type": "string" },
                "nom": { "type": "string" },
                "race": { "type": "string" },
                "sexe": { "type": "string" },
                "annee_naissance": { "type": "integer" },
                "generation": {
                    "type": "integer",
                    "description": "0 pour le sujet, 1 pour ses parents, etc."
                },
                "pere": { "$ref": "#/components/schemas/Noeud" },
                "mere": { "$ref": "#/components/schemas/Noeud" }
            }
        },
        "Pagination": {
            "type": "object",
            "required": ["total", "limite", "offset"],
            "properties": {
                "total": { "type": "integer", "description": "Lignes correspondant aux filtres, avant découpage." },
                "limite": { "type": "integer" },
                "offset": { "type": "integer" },
                "suivant": {
                    "type": ["string", "null"],
                    "description": "URI de la page suivante, filtres conservés. Nulle sur la \
                                    dernière page ou lorsque la profondeur maximale est atteinte."
                },
                "precedent": {
                    "type": ["string", "null"],
                    "description": "URI de la page précédente. Nulle sur la première page."
                },
                "note": {
                    "type": "string",
                    "description": "Présent seulement quand la sélection dépasse la profondeur \
                                    de pagination : le reste n'est pas atteignable par cette route."
                }
            }
        },
        "Resume": {
            "type": "object",
            "description": "Vue allégée servie par les listes. La fiche complète s'obtient \
                            sur /v1/equides/{id} — aucun lien n'est répété par ligne, il se \
                            déduit de l'identifiant.",
            "required": ["id", "nom", "race", "sexe"],
            "properties": {
                "id": { "type": "string", "example": "Z4ogLhlkS2CeUdq0bZ0YFw" },
                "nom": { "type": "string", "example": "QABALAH MERCURY" },
                "race": { "type": "string", "example": "Trotteur Francais" },
                "sexe": { "type": "string", "enum": ["Femelle", "Male", "Hongre", "Indeter"] },
                "robe": { "type": "string", "example": "Bai" },
                "annee_naissance": { "type": "integer", "example": 2026 }
            }
        },
        "PageEquides": {
            "type": "object",
            "required": ["donnees", "pagination"],
            "properties": {
                "donnees": {
                    "type": "array",
                    "description": "Résumés par défaut, fiches complètes si `vue=complet`.",
                    "items": {
                        "oneOf": [
                            { "$ref": "#/components/schemas/Resume" },
                            { "$ref": "#/components/schemas/Equide" }
                        ]
                    }
                },
                "pagination": { "$ref": "#/components/schemas/Pagination" }
            }
        },
        "Suggestions": {
            "type": "object",
            "required": ["q", "limite", "resultats"],
            "properties": {
                "q": {
                    "type": "string",
                    "description": "Requête telle que reçue, pour que le client sache à quoi \
                                    la réponse répond — les frappes arrivent dans le désordre."
                },
                "limite": { "type": "integer" },
                "resultats": {
                    "type": "array",
                    "description": "De la suggestion la plus pertinente à la moins.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "nom": { "type": "string" },
                            "race": { "type": "string" },
                            "sexe": { "type": "string" },
                            "annee_naissance": { "type": "integer" },
                            "score": {
                                "type": "number",
                                "description": "Pertinence dans [0, 1] ; 1 pour une \
                                                correspondance exacte de mot."
                            }
                        }
                    }
                }
            }
        },
        "Pedigree": {
            "type": "object",
            "required": ["generations", "noeuds", "noeuds_theoriques", "arbre"],
            "properties": {
                "generations": { "type": "integer", "description": "Profondeur demandée." },
                "noeuds": {
                    "type": "integer",
                    "description": "Ancêtres réellement connus, sujet compris."
                },
                "noeuds_theoriques": {
                    "type": "integer",
                    "description": "Ce qu'un pedigree complet compterait à cette profondeur. \
                                    L'écart avec `noeuds` mesure les lacunes de l'ascendance."
                },
                "arbre": { "$ref": "#/components/schemas/Noeud" }
            }
        },
        "Produit": {
            "type": "object",
            "description": "Un produit direct, décrit comme un nœud de pedigree : \
                            mêmes champs, même sens.",
            "required": ["id", "nom", "race", "sexe", "fiche_disponible"],
            "properties": {
                "id": { "type": "string", "example": "QylKqa2SStCCxjmFVYNvvg" },
                "nom": { "type": "string", "example": "PAMPA D'ARPO" },
                "race": { "type": "string" },
                "sexe": { "type": "string", "example": "Femelle" },
                "annee_naissance": { "type": "integer", "example": 2004 },
                "fiche_disponible": {
                    "type": "boolean",
                    "description": "Toujours vrai : un produit est une fiche du jeu."
                }
            }
        },
        "Descendance": {
            "type": "object",
            "required": ["parent", "total", "limite", "offset", "donnees"],
            "properties": {
                "parent": { "$ref": "#/components/schemas/Reference" },
                "total": {
                    "type": "integer",
                    "description": "Produits directs connus, avant découpage."
                },
                "limite": { "type": "integer" },
                "offset": { "type": "integer" },
                "donnees": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Produit" }
                }
            }
        },
        "Referentiel": {
            "type": "object",
            "required": ["dimension", "nombre_de_modalites", "donnees"],
            "properties": {
                "dimension": { "type": "string", "example": "races" },
                "nombre_de_modalites": {
                    "type": "integer",
                    "description": "Total réel, y compris les modalités écartées par la troncature."
                },
                "note": {
                    "type": "string",
                    "description": "Présent seulement quand la liste est tronquée. Une troncature \
                                    silencieuse laisserait croire à une liste exhaustive."
                },
                "donnees": {
                    "type": "array",
                    "description": "De la modalité la plus fréquente à la plus rare, sauf pour \
                                    `annees_naissance`, rendu par ordre chronologique.",
                    "items": { "$ref": "#/components/schemas/Modalite" }
                }
            }
        },
        "Modalite": {
            "type": "object",
            "description": "Une valeur admise par un filtre, avec son effectif.",
            "required": ["valeur", "nombre"],
            "properties": {
                "valeur": { "type": "string", "example": "Trotteur Francais" },
                "nombre": { "type": "integer" }
            }
        },
        "Stats": {
            "type": "object",
            "properties": {
                "total": { "type": "integer" },
                "avec_performances": { "type": "integer" },
                "sans_performances": { "type": "integer" },
                "avec_filiation": {
                    "type": "integer",
                    "description": "Équidés dont au moins un parent est renseigné."
                },
                "sans_filiation": { "type": "integer" },
                "avec_record": {
                    "type": "integer",
                    "description": "Fiches portant un chrono de course déclaré."
                },
                "avec_statut_reproducteur": {
                    "type": "integer",
                    "description": "Fiches portant un statut déclaré à la reproduction, \
                                    toutes modalités confondues."
                },
                "par_sexe": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Tranche" }
                },
                "nombre_de_races": { "type": "integer" },
                "nombre_de_robes": { "type": "integer" },
                "nombre_de_disciplines": { "type": "integer" },
                "annee_naissance_min": { "type": ["integer", "null"] },
                "annee_naissance_max": { "type": ["integer", "null"] },
                "avertissement": {
                    "type": "string",
                    "description": "Rappel du sens de « sans performances », que le seul \
                                    effectif induirait en erreur."
                }
            }
        },
        "Repartition": {
            "type": "object",
            "required": ["dimension", "effectif_filtre", "effectif_affiche", "donnees"],
            "properties": {
                "dimension": { "type": "string" },
                "effectif_filtre": {
                    "type": "integer",
                    "description": "Lignes retenues par les filtres, avant découpage."
                },
                "effectif_affiche": {
                    "type": "integer",
                    "description": "Somme des effectifs renvoyés. Inférieure à `effectif_filtre` \
                                    si `limite` tronque la liste, ou si des lignes n'ont pas de \
                                    valeur pour cette dimension."
                },
                "donnees": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Tranche" }
                }
            }
        },
        "Meta": {
            "type": "object",
            "required": ["version_api", "jeu_de_donnees", "remarques"],
            "properties": {
                "version_api": { "type": "string" },
                "jeu_de_donnees": { "$ref": "#/components/schemas/JeuDeDonnees" },
                "remarques": {
                    "type": "array",
                    "description": "Précisions sur la donnée, son régime de réutilisation et ses \
                                    défauts connus, que la fiche du producteur ne porte pas.",
                    "items": { "type": "string" }
                }
            }
        },
        "JeuDeDonnees": {
            "type": "object",
            "description": "Provenance de l'extraction servie. `source` et `ingere_le` sont ce \
                            que la mention obligatoire de réutilisation doit citer.",
            "properties": {
                "producteur": { "type": "string" },
                "source": { "type": "string" },
                "licence": { "type": "string" },
                "url_modele": {
                    "type": "string",
                    "description": "Gabarit reconstruisant l'URL publique d'une fiche."
                },
                "ingere_le": { "type": "string", "description": "Date d'ingestion, ISO 8601." },
                "fichier_source": {
                    "type": "string",
                    "description": "Nom du fichier d'extraction ingéré, celui que \
                                    `empreinte_source` et `octets_source` décrivent.",
                    "example": "chevaux.jsonl"
                },
                "octets_source": {
                    "type": "integer",
                    "description": "Taille exacte du fichier ingéré."
                },
                "algorithme_empreinte": {
                    "type": "string",
                    "description": "Fonction de hachage employée pour `empreinte_source`. \
                                    Publiée pour que l'empreinte soit reproductible sans \
                                    avoir à deviner l'algorithme.",
                    "example": "blake3"
                },
                "empreinte_source": {
                    "type": "string",
                    "description": "Empreinte du fichier ingéré, octet pour octet — pas celle \
                                    d'un export normalisé. Elle identifie exactement \
                                    l'extraction servie."
                },
                "lignes": { "type": "integer" },
                "anomalies": { "$ref": "#/components/schemas/Anomalies" }
            }
        },
        "Anomalies": {
            "type": "object",
            "description": "Défauts de la source, dénombrés à l'ingestion et conservés tels \
                            quels : cette API est un miroir fidèle, pas un correcteur.",
            "properties": {
                "annee_naissance_absente": { "type": "integer" },
                "annee_naissance_min": { "type": ["integer", "null"] },
                "annee_naissance_max": { "type": ["integer", "null"] },
                "robe_absente": { "type": "integer" },
                "sans_filiation": { "type": "integer" },
                "parents_references": {
                    "type": "integer",
                    "description": "Identifiants de parents distincts cités par au moins une fiche."
                },
                "parents_pendants": {
                    "type": "integer",
                    "description": "Ceux d'entre eux qui sont absents du fichier : le lien est \
                                    rompu, et exposé comme une absence de parent. Dénombrés par \
                                    identifiant, pas par citation — un étalon absent cité mille \
                                    fois compte pour un."
                },
                "references_de_parents": {
                    "type": "integer",
                    "description": "Emplacements `pere`, `mere` ou `pere_de_mere` portant un \
                                    identifiant, toutes fiches confondues."
                },
                "references_pendantes": {
                    "type": "integer",
                    "description": "Ceux d'entre eux dont la cible manque. Toujours supérieur ou \
                                    égal à `parents_pendants` : c'est le même défaut compté par \
                                    citation plutôt que par identifiant."
                },
                "identifiants_illisibles": { "type": "integer" },
                "slugs_non_deductibles": { "type": "integer" }
            }
        },
        "Decouverte": {
            "type": "object",
            "description": "Document de découverte servi à la racine sur `Accept: application/json`.",
            "properties": {
                "nom": { "type": "string" },
                "version": { "type": "string" },
                "description": { "type": "string" },
                "affiliation": { "type": "string" },
                "lignes": { "type": "integer" },
                "documentation": { "type": "string" },
                "openapi": { "type": "string" },
                "endpoints": {
                    "type": "object",
                    "description": "Routes servies, par usage.",
                    "additionalProperties": { "type": "string" }
                },
                "gabarits_url": {
                    "type": "object",
                    "description": "Publiés ici une fois pour toutes : aucune réponse ne répète \
                                    un lien déductible d'un identifiant.",
                    "additionalProperties": { "type": "string" }
                },
                "licence_donnees": { "type": "string" },
                "source": { "type": "string" }
            }
        },
        "Sante": {
            "type": "object",
            "required": ["statut", "lignes", "version"],
            "properties": {
                "statut": { "type": "string", "enum": ["ok", "indisponible"] },
                "lignes": {
                    "type": "integer",
                    "description": "Lignes chargées en mémoire. Le service n'est prêt qu'au-delà de zéro."
                },
                "version": { "type": "string" }
            }
        },
        "Probleme": {
            "type": "object",
            "description": "Corps d'erreur conforme à la RFC 9457 (« Problem Details for HTTP \
                            APIs »), servi en `application/problem+json`. Les champs `parametre` \
                            et `indice` sont des extensions, que la RFC autorise.",
            "required": ["type", "title", "status", "detail"],
            "properties": {
                "type": {
                    "type": "string",
                    "description": "URI du type de problème, relative — le client la résout \
                                    contre l'URI de la requête. Elle pointe vers la section \
                                    correspondante de la documentation.",
                    "example": "/#erreur-parametre-invalide"
                },
                "title": {
                    "type": "string",
                    "description": "Résumé lisible, stable pour un `type` donné.",
                    "example": "Paramètre invalide"
                },
                "status": { "type": "integer", "example": 400 },
                "detail": {
                    "type": "string",
                    "description": "Explication propre à cette occurrence.",
                    "example": "paramètre inconnu `rase`"
                },
                "parametre": {
                    "type": "string",
                    "description": "Extension : paramètre fautif, quand l'erreur en désigne un.",
                    "example": "rase"
                },
                "indice": {
                    "type": "string",
                    "description": "Extension : piste de correction — valeurs admises, \
                                    référentiel à consulter, ou paramètre le plus proche en cas \
                                    de faute de frappe.",
                    "example": "vouliez-vous dire `race` ?"
                },
                "documentation": { "type": "string", "example": "https://docs.api-equides.org" }
            }
        },
        "Tranche": {
            "type": "object",
            "description": "Une modalité et son effectif dans la sélection.",
            "required": ["valeur", "nombre"],
            "properties": {
                "valeur": { "type": "string" },
                "nombre": { "type": "integer" }
            }
        }
    })
}

fn reponse_erreur(description: &str) -> Value {
    json!({
        "description": description,
        "content": { "application/problem+json": {
            "schema": { "$ref": "#/components/schemas/Probleme" }
        }}
    })
}

fn reponses(propres: Value) -> Value {
    let mut o = match propres {
        Value::Object(o) => o,
        _ => serde_json::Map::new(),
    };
    o.entry("304").or_insert(json!({
        "description": "Contenu inchangé depuis l'`ETag` fourni en `If-None-Match`. \
                        Aucun corps, et rien n'est recalculé côté service."
    }));
    o.entry("429").or_insert(reponse_erreur(
        "Quota de débit dépassé pour l'adresse appelante. `Retry-After` indique le délai.",
    ));
    o.entry("503").or_insert(reponse_erreur(
        "Capacité de traitement simultané atteinte, délai maximal dépassé, ou données \
         non chargées. `Retry-After` indique le délai.",
    ));
    Value::Object(o)
}

fn erreurs() -> Value {
    erreur::CATALOGUE
        .iter()
        .map(|t| {
            json!({
                "ancre": t.ancre(),
                "uri": t.uri,
                "titre": t.titre,
                "statut": t.statut.as_u16(),
                "quand": t.quand,
            })
        })
        .collect()
}

pub fn document(store: &Store) -> Value {
    let filtres = params_filtre(store);
    let mut params_recherche = filtres.clone();
    params_recherche.extend([
        param(
            "tri",
            "Ordre : `naturel` (ordre du fichier source, défaut), `nom`, `annee`. \
             Préfixer par `-` inverse le sens. Les millésimes absents sont toujours placés en fin de liste.",
            &json!({ "type": "string", "enum": ["naturel", "nom", "annee", "-naturel", "-nom", "-annee"], "default": "naturel" }),
            &json!("-annee"),
        ),
        param(
            "vue",
            "Niveau de détail des lignes renvoyées : `resume` (défaut, ~180 octets par \
             ligne) ou `complet` (fiche entière, ~1,1 Kio par ligne).",
            &json!({ "type": "string", "enum": ["resume", "complet"], "default": "resume" }),
            &json!("resume"),
        ),
        param(
            "limite",
            &format!(
                "Lignes par page (1 à {LIMITE_MAX}, défaut {LIMITE_DEFAUT}). \
                 Une valeur hors bornes est refusée par un `400`."
            ),
            &json!({ "type": "integer", "minimum": 1, "maximum": LIMITE_MAX, "default": LIMITE_DEFAUT }),
            &json!(20),
        ),
        param(
            "offset",
            &format!(
                "Décalage, plafonné à {OFFSET_MAX} ; au-delà, un `400`. La pagination profonde \
                 dégrade le service : pour parcourir le jeu entier, téléchargez le fichier publié."
            ),
            &json!({ "type": "integer", "minimum": 0, "maximum": OFFSET_MAX, "default": 0 }),
            &json!(0),
        ),
    ]);

    let mut params_repartition = filtres;
    params_repartition.extend([
        param(
            "dimension",
            "Dimension à ventiler : `race`, `robe`, `sexe`, `statut_reproducteur`, \
             `discipline`, `annee_naissance`.",
            &json!({
                "type": "string",
                "enum": ["race", "robe", "sexe", "statut_reproducteur", "discipline", "annee_naissance"],
                "default": "race"
            }),
            &json!("race"),
        ),
        param(
            "limite",
            &format!(
                "Nombre de modalités renvoyées, les plus fournies d'abord (1 à \
                 {MODALITES_REPARTITION_MAX}, défaut {MODALITES_REPARTITION_DEFAUT}). \
                 Une valeur hors bornes est ramenée au plafond, non refusée. \
                 Sans effet sur `annee_naissance`, rendue en entier."
            ),
            &json!({
                "type": "integer",
                "minimum": 1,
                "maximum": MODALITES_REPARTITION_MAX,
                "default": MODALITES_REPARTITION_DEFAUT
            }),
            &json!(20),
        ),
    ]);

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "API Équidés",
            "version": crate::VERSION,
            "summary": "Interrogation des données publiques des équidés enregistrés en France depuis 1976.",
            "description": format!(
                "API REST ouverte sur les fiches publiques d'équidés de l'IFCE, avec généalogie \
                 et indices de performance.\n\n\
                 **Service indépendant, non affilié à l'IFCE.** La donnée servie est une \
                 extraction datée ; les fiches en ligne font seules autorité.\n\n\
                 **Données** : {source}, {lignes} fiches. Licence : {licence}. Mentionnez la \
                 source et la date de mise à jour, exposées par `/v1/meta`. Aucune donnée à \
                 caractère personnel n'est exposée.\n\n\
                 **Sans authentification** : ni clé, ni jeton, ni compte.\n\n\
                 ## Limites\n\n\
                 Débit {debit} unités par seconde et par adresse, rafale {rafale} — une requête \
                 ordinaire coûte une unité, les réponses volumineuses davantage. Concurrence \
                 {concurrence}, délai {delai} s. Pagination {limite_max} lignes, profondeur \
                 {offset_max}. Pedigree {generations_max} générations. Requête {params_max} \
                 paramètres et {query_max} octets. Référentiels tronqués à {modalites_max} \
                 modalités.\n\n\
                 Les réponses portent un `ETag` et un `Cache-Control` d'une heure ; une \
                 revalidation `If-None-Match` renvoie `304` sans rien recalculer. Les erreurs \
                 suivent la RFC 9457, en `application/problem+json`.",
                source = store.meta.source,
                lignes = store.meta.lignes,
                licence = store.meta.licence,
                debit = crate::api::protection::DEBIT_PAR_SECONDE,
                rafale = crate::api::protection::RAFALE,
                concurrence = crate::api::protection::concurrence_max(),
                delai = crate::api::DELAI_MAX.as_secs(),
                limite_max = LIMITE_MAX,
                offset_max = OFFSET_MAX,
                params_max = crate::api::protection::PARAMS_MAX,
                query_max = crate::api::protection::QUERY_MAX,
                generations_max = GENERATIONS_MAX,
                modalites_max = MODALITES_MAX,
            ),
            "license": { "name": "MIT (code)", "identifier": "MIT" },
            "contact": { "name": "Fiches d'origine", "url": "https://infochevaux.ifce.fr" }
        },
        "servers": [
            { "url": "https://api-equides.org", "description": "Instance publique" },
            { "url": "/", "description": "Ce serveur" }
        ],
        "tags": [
            { "name": "Équidés", "description": "Recherche et consultation." },
            { "name": "Généalogie", "description": "Ascendance et descendance." },
            { "name": "Référentiels", "description": "Valeurs admises par les filtres." },
            { "name": "Statistiques", "description": "Agrégats et répartitions." },
            { "name": "Service", "description": "Découverte, métadonnées et exploitation." }
        ],
        "x-erreurs": erreurs(),
        "paths": {
            "/v1/equides": {
                "get": {
                    "tags": ["Équidés"],
                    "summary": "Rechercher des équidés",
                    "description": "Les filtres d'une même dimension se combinent en OU, \
                                    les dimensions différentes en ET.\n\n\
                                    Les lignes sont renvoyées en **résumé** par défaut ; \
                                    `vue=complet` sert les fiches entières.\n\n\
                                    Aucune réponse ne porte de lien déductible d'un \
                                    identifiant : les gabarits d'URL sont publiés par `GET /`.",
                    "parameters": params_recherche,
                    "responses": reponses(json!({
                        "200": {
                            "description": "Page de résultats.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/PageEquides" }
                            }}
                        },
                        "400": reponse_erreur(
                            "Paramètre inconnu, valeur hors bornes, ou modalité que le \
                             référentiel n'admet pas."
                        )
                    }))
                }
            },
            "/v1/search": {
                "get": {
                    "tags": ["Équidés"],
                    "summary": "Autocomplétion sur le nom",
                    "description": "Renvoie des suggestions **classées par pertinence**, pour \
                                    alimenter un champ de saisie.\n\n\
                                    À distinguer de `/v1/equides?nom=…`, qui filtre et pagine \
                                    sans classer. Ici le coût est borné par `limite` et non par \
                                    la popularité du préfixe : la réponse doit arriver avant la \
                                    frappe suivante.",
                    "parameters": [
                        { "name": "q", "in": "query", "required": true,
                          "description": "Début de nom, insensible à la casse et aux accents. \
                                          Obligatoire et non vide.\n\n\
                                          Les espaces comptent : chaque mot déjà saisi doit se \
                                          retrouver dans le nom, seul le dernier est traité \
                                          comme un début de mot. `INVICTUS DU F` ne suggère \
                                          donc que des noms portant `INVICTUS`, `DU`, et un mot \
                                          commençant par `F`.",
                          "schema": { "type": "string", "minLength": 1 },
                          "example": "invictus du f" },
                        { "name": "limite", "in": "query", "required": false,
                          "description": format!(
                              "Nombre de suggestions (1 à {LIMITE_MAX}, défaut \
                               {SUGGESTIONS_DEFAUT}). Une valeur hors bornes est ramenée au \
                               plafond, non refusée."
                          ),
                          "schema": {
                              "type": "integer",
                              "minimum": 1,
                              "maximum": LIMITE_MAX,
                              "default": SUGGESTIONS_DEFAUT
                          },
                          "example": 10 }
                    ],
                    "responses": reponses(json!({
                        "200": {
                            "description": "Suggestions, de la plus pertinente à la moins.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Suggestions" }
                            }}
                        },
                        "400": reponse_erreur("`q` manquant ou vide, ou paramètre inconnu.")
                    }))
                }
            },
            "/v1/equides/{id}": {
                "get": {
                    "tags": ["Équidés"],
                    "summary": "Consulter un équidé",
                    "parameters": [{
                        "name": "id", "in": "path", "required": true,
                        "description": "Identifiant renvoyé par la recherche : celui de l'IFCE, \
                                        22 caractères en base64 URL.",
                        "schema": { "type": "string" },
                        "example": "Z4ogLhlkS2CeUdq0bZ0YFw"
                    }],
                    "responses": reponses(json!({
                        "200": {
                            "description": "L'équidé demandé.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Equide" }
                            }}
                        },
                        "404": reponse_erreur(
                            "Identifiant inconnu ou mal formé — les deux cas donnent la même \
                             réponse."
                        )
                    }))
                }
            },
            "/v1/equides/{id}/pedigree": {
                "get": {
                    "tags": ["Généalogie"],
                    "summary": "Ascendance sur N générations",
                    "description": "En REST naïf, remonter un pedigree coûterait une requête par \
                                    ancêtre — 31 allers-retours sur cinq générations. La filiation \
                                    étant résolue en indices de ligne à l'ingestion, cet endpoint \
                                    répond en une seule requête cacheable, au coût borné.\n\n\
                                    Un parent référencé mais absent du jeu interrompt la branche : \
                                    l'écart entre `noeuds` et `noeuds_theoriques` mesure ces \
                                    lacunes.",
                    "parameters": [
                        { "name": "id", "in": "path", "required": true,
                          "schema": { "type": "string" },
                          "example": "Z4ogLhlkS2CeUdq0bZ0YFw" },
                        { "name": "generations", "in": "query", "required": false,
                          "description": format!(
                              "Profondeur, {GENERATIONS_DEFAUT} par défaut, {GENERATIONS_MAX} au \
                               maximum ; au-delà, un `400`. Un arbre double de taille à chaque \
                               rang : {GENERATIONS_MAX} générations représentent déjà jusqu'à \
                               1 023 ancêtres."
                          ),
                          "schema": {
                              "type": "integer",
                              "minimum": 0,
                              "maximum": GENERATIONS_MAX,
                              "default": GENERATIONS_DEFAUT
                          },
                          "example": 5 }
                    ],
                    "responses": reponses(json!({
                        "200": {
                            "description": "Arbre d'ascendance. `noeuds` compte les ancêtres \
                                            réellement connus, `noeuds_theoriques` ceux qu'un \
                                            pedigree complet compterait.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Pedigree" }
                            }}
                        },
                        "400": reponse_erreur("Profondeur hors bornes."),
                        "404": reponse_erreur("Identifiant inconnu.")
                    }))
                }
            },
            "/v1/equides/{id}/descendance": {
                "get": {
                    "tags": ["Généalogie"],
                    "summary": "Produits directs",
                    "description": "Réponse symétrique au pedigree, servie par un index inverse \
                                    bâti au chargement.\n\n\
                                    Ne renvoie que les produits **directs** : la descendance \
                                    complète se parcourt en suivant les identifiants de proche \
                                    en proche.",
                    "parameters": [
                        { "name": "id", "in": "path", "required": true,
                          "schema": { "type": "string" },
                          "example": "Z4ogLhlkS2CeUdq0bZ0YFw" },
                        { "name": "limite", "in": "query", "required": false,
                          "description": format!(
                              "Produits par page (1 à {LIMITE_MAX}, défaut {DESCENDANCE_DEFAUT}). \
                               Une valeur hors bornes est ramenée au plafond, non refusée."
                          ),
                          "schema": {
                              "type": "integer",
                              "minimum": 1,
                              "maximum": LIMITE_MAX,
                              "default": DESCENDANCE_DEFAUT
                          },
                          "example": 50 },
                        { "name": "offset", "in": "query", "required": false,
                          "description": "Décalage dans la liste des produits. Un décalage \
                                          au-delà du total renvoie une page vide, `total` \
                                          restant le compte réel.",
                          "schema": { "type": "integer", "minimum": 0, "default": 0 },
                          "example": 0 }
                    ],
                    "responses": reponses(json!({
                        "200": {
                            "description": "Descendance directe, page courante.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Descendance" }
                            }}
                        },
                        "400": reponse_erreur("Paramètre inconnu ou non entier."),
                        "404": reponse_erreur("Identifiant inconnu.")
                    }))
                }
            },
            "/v1/referentiels/{dimension}": {
                "get": {
                    "tags": ["Référentiels"],
                    "summary": "Lister les valeurs admises par un filtre",
                    "description": format!(
                        "Sans cette liste, un intégrateur ne peut pas deviner que la source \
                         écrit `Trotteur Francais` sans cédille ni accent, ni que le sexe se \
                         dit `Femelle` et non `F`.\n\n\
                         Les modalités sont rendues de la plus fréquente à la plus rare, et la \
                         liste est tronquée à {MODALITES_MAX} entrées — `nombre_de_modalites` \
                         donne alors le total réel et `note` signale la troncature. \
                         `annees_naissance` fait exception : rendu en entier, par ordre \
                         chronologique."
                    ),
                    "parameters": [{
                        "name": "dimension", "in": "path", "required": true,
                        "description": format!(
                            "Référentiel demandé. `races` liste les {races} libellés développés \
                             portés par les fiches ; `races_lien` les {races_lien} codes abrégés \
                             portés par les liens de filiation. Ce sont deux nomenclatures \
                             distinctes, et seule la première est admise par le filtre `race=`.",
                            races = store.races.len(),
                            races_lien = store.races_lien.len(),
                        ),
                        "schema": { "type": "string",
                                    "enum": ["races", "robes", "sexes", "statuts_reproducteur", "disciplines", "codes_indice", "races_lien", "annees_naissance"] },
                        "example": "races"
                    }],
                    "responses": reponses(json!({
                        "200": {
                            "description": "Modalités et effectifs, du plus fréquent au plus rare.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Referentiel" }
                            }}
                        },
                        "400": reponse_erreur("Référentiel inconnu.")
                    }))
                }
            },
            "/v1/stats": {
                "get": {
                    "tags": ["Statistiques"],
                    "summary": "Vue d'ensemble du jeu de données",
                    "description": "Effectifs globaux, sans filtre. Pour ventiler une sélection, \
                                    voir `/v1/stats/repartition`.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Effectifs globaux.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Stats" }
                            }}
                        }
                    }))
                }
            },
            "/v1/stats/repartition": {
                "get": {
                    "tags": ["Statistiques"],
                    "summary": "Ventiler une sélection selon une dimension",
                    "description": "Accepte les mêmes filtres que la recherche : \
                                    la répartition porte sur la sélection filtrée.\n\n\
                                    De quoi peupler des facettes sans rapatrier les lignes : \
                                    seuls les effectifs traversent le réseau.",
                    "parameters": params_repartition,
                    "responses": reponses(json!({
                        "200": {
                            "description": "Effectifs par modalité.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Repartition" }
                            }}
                        },
                        "400": reponse_erreur("Dimension ou filtre invalide.")
                    }))
                }
            },
            "/v1/meta": {
                "get": {
                    "tags": ["Service"],
                    "summary": "Provenance, licence et qualité de la donnée servie",
                    "description": "La réutilisation impose de mentionner la source et la date \
                                    de mise à jour : `source`, `ingere_le` et `empreinte_source` \
                                    y pourvoient.\n\n\
                                    Les `remarques` énoncent les défauts connus de la source et \
                                    le régime de réutilisation applicable.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Métadonnées du jeu de données chargé.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Meta" }
                            }}
                        }
                    }))
                }
            },
            "/": {
                "get": {
                    "tags": ["Service"],
                    "summary": "Document de découverte",
                    "description": "Le point d'entrée : routes servies, gabarits d'URL, \
                                    licence et provenance. Un client qui ne connaît que cette \
                                    URL y trouve de quoi construire toutes les autres. La \
                                    documentation destinée aux humains vit sur \
                                    <https://docs.api-equides.org>.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Document de découverte.",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/Decouverte" }
                                }
                            }
                        }
                    }))
                }
            },
            "/openapi.json": {
                "get": {
                    "tags": ["Service"],
                    "summary": "Description OpenAPI de ce service",
                    "description": "La page de documentation est rendue depuis ce document : \
                                    l'une ne peut pas diverger de l'autre.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Document OpenAPI 3.1.",
                            "content": { "application/json": {
                                "schema": { "type": "object" }
                            }}
                        }
                    }))
                }
            },
            "/healthz": {
                "get": {
                    "tags": ["Service"],
                    "summary": "Sonde de vivacité",
                    "description": "Reste sur le port public : un répartiteur de charge doit \
                                    pouvoir l'interroger sans accès au plan d'administration. \
                                    La télémétrie Prometheus, elle, n'est servie que sur \
                                    l'écoute privée.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Service opérationnel.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Sante" }
                            }}
                        },
                        "503": {
                            "description": "Aucune donnée chargée : le service n'est pas prêt.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Sante" }
                            }}
                        }
                    }))
                }
            },
            "/readyz": {
                "get": {
                    "tags": ["Service"],
                    "summary": "Sonde de disponibilité",
                    "description": "Identique à `/healthz` : l'image de données étant chargée \
                                    avant la première écoute, vivacité et disponibilité se \
                                    confondent ici. Les deux routes existent pour les \
                                    orchestrateurs qui distinguent les deux sondes.",
                    "responses": reponses(json!({
                        "200": {
                            "description": "Service prêt.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Sante" }
                            }}
                        },
                        "503": {
                            "description": "Aucune donnée chargée.",
                            "content": { "application/json": {
                                "schema": { "$ref": "#/components/schemas/Sante" }
                            }}
                        }
                    }))
                }
            },
        },
        "components": { "schemas": schemas() }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::params::PARAMS_FILTRE;
    use crate::store::tests::store_test;

    #[test]
    fn le_document_est_bien_forme() {
        let s = store_test();
        let doc = document(&s);
        assert_eq!(doc["openapi"], "3.1.0");
        assert!(doc["paths"]["/v1/equides"]["get"]["parameters"].is_array());
        assert!(doc["components"]["schemas"]["Equide"].is_object());
        assert!(serde_json::to_string(&doc).is_ok());
    }

    #[test]
    fn les_parametres_documentes_sont_exactement_ceux_acceptes() {
        let s = store_test();
        let doc = document(&s);

        let documentes = |chemin: &str| -> Vec<String> {
            doc["paths"][chemin]["get"]["parameters"]
                .as_array()
                .unwrap_or_else(|| panic!("paramètres absents pour {chemin}"))
                .iter()
                .filter(|p| p["in"] == "query")
                .map(|p| p["name"].as_str().unwrap().to_string())
                .collect()
        };

        let mut attendus: Vec<String> = PARAMS_FILTRE
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        attendus.extend(["tri", "limite", "offset", "vue"].map(String::from));
        let mut obtenus = documentes("/v1/equides");
        attendus.sort();
        obtenus.sort();
        assert_eq!(
            obtenus, attendus,
            "les paramètres documentés de /v1/equides divergent de ceux acceptés"
        );

        let mut attendus: Vec<String> = PARAMS_FILTRE
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        attendus.extend(["dimension", "limite"].map(String::from));
        let mut obtenus = documentes("/v1/stats/repartition");
        attendus.sort();
        obtenus.sort();
        assert_eq!(
            obtenus, attendus,
            "paramètres divergents sur /v1/stats/repartition"
        );

        assert_eq!(documentes("/v1/search"), ["q", "limite"]);
        assert_eq!(documentes("/v1/equides/{id}/pedigree"), ["generations"]);
        assert_eq!(
            documentes("/v1/equides/{id}/descendance"),
            ["limite", "offset"]
        );
    }

    #[test]
    fn les_chemins_documentes_sont_ceux_servis() {
        let s = store_test();
        let doc = document(&s);
        let mut documentes: Vec<&str> = doc["paths"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut servis = vec![
            "/",
            "/v1/equides",
            "/v1/search",
            "/v1/equides/{id}",
            "/v1/equides/{id}/pedigree",
            "/v1/equides/{id}/descendance",
            "/v1/referentiels/{dimension}",
            "/v1/stats",
            "/v1/stats/repartition",
            "/v1/meta",
            "/openapi.json",
            "/healthz",
            "/readyz",
        ];
        documentes.sort_unstable();
        servis.sort_unstable();
        assert_eq!(documentes, servis);
    }

    #[test]
    fn aucune_reference_de_schema_pendante() {
        let s = store_test();
        let doc = document(&s);
        let definis: Vec<String> = doc["components"]["schemas"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();

        fn collecter(v: &Value, refs: &mut Vec<String>) {
            match v {
                Value::Object(o) => {
                    for (k, val) in o {
                        if k == "$ref" {
                            if let Some(r) = val.as_str() {
                                refs.push(
                                    r.trim_start_matches("#/components/schemas/").to_string(),
                                );
                            }
                        } else {
                            collecter(val, refs);
                        }
                    }
                }
                Value::Array(a) => a.iter().for_each(|x| collecter(x, refs)),
                _ => {}
            }
        }
        let mut refs = Vec::new();
        collecter(&doc, &mut refs);
        assert!(
            !refs.is_empty(),
            "le document devrait référencer des schémas"
        );
        for r in refs {
            assert!(
                definis.contains(&r),
                "référence pendante vers le schéma « {r} »"
            );
        }
    }

    #[test]
    fn aucun_schema_orphelin() {
        let s = store_test();
        let doc = document(&s);
        let serialise = serde_json::to_string(&doc).unwrap();
        for nom in doc["components"]["schemas"].as_object().unwrap().keys() {
            assert!(
                serialise.contains(&format!("#/components/schemas/{nom}")),
                "schéma « {nom} » défini mais jamais référencé"
            );
        }
    }

    #[test]
    fn les_exemples_viennent_du_jeu_de_donnees_charge() {
        let s = store_test();
        let doc = document(&s);
        let params = doc["paths"]["/v1/equides"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let race = params.iter().find(|p| p["name"] == "race").unwrap();
        assert!(
            ["Trotteur Francais", "Pur Sang"].contains(&race["example"].as_str().unwrap()),
            "l'exemple doit être une race réellement présente, trouvé {:?}",
            race["example"]
        );
    }

    #[test]
    fn les_erreurs_sont_decrites_au_type_de_media_reel() {
        let s = store_test();
        let doc = document(&s);
        let mut vues = 0;
        for (chemin, item) in doc["paths"].as_object().unwrap() {
            let reponses = item["get"]["responses"].as_object().unwrap();
            for (code, r) in reponses {
                if !code.starts_with('4') && !code.starts_with('5') {
                    continue;
                }
                if r["content"]["application/json"].is_object() {
                    continue;
                }
                vues += 1;
                assert!(
                    r["content"]["application/problem+json"]["schema"]["$ref"]
                        == "#/components/schemas/Probleme",
                    "{chemin} {code} : une erreur doit être servie en application/problem+json"
                );
            }
        }
        assert!(vues > 0, "aucune réponse d'erreur décrite");
    }

    #[test]
    fn chaque_operation_annonce_les_reponses_globales() {
        let s = store_test();
        let doc = document(&s);
        for (chemin, item) in doc["paths"].as_object().unwrap() {
            let reponses = item["get"]["responses"].as_object().unwrap();
            for code in ["304", "429", "503"] {
                assert!(
                    reponses.contains_key(code),
                    "{chemin} n'annonce pas le code {code}, que tout endpoint peut renvoyer"
                );
            }
        }
    }

    #[test]
    fn le_catalogue_des_erreurs_est_publie() {
        let s = store_test();
        let doc = document(&s);
        let publiees = doc["x-erreurs"].as_array().unwrap();
        assert_eq!(publiees.len(), crate::api::erreur::CATALOGUE.len());
        for t in crate::api::erreur::CATALOGUE {
            assert!(
                publiees.iter().any(|p| p["uri"] == t.uri),
                "type absent du document : {}",
                t.uri
            );
        }
    }

    #[test]
    fn les_limites_annoncees_sont_celles_appliquees() {
        let s = store_test();
        let doc = document(&s);
        let texte = doc["info"]["description"].as_str().unwrap();
        for attendu in [
            crate::api::protection::DEBIT_PAR_SECONDE.to_string(),
            crate::api::protection::RAFALE.to_string(),
            crate::api::protection::concurrence_max().to_string(),
            crate::api::DELAI_MAX.as_secs().to_string(),
            LIMITE_MAX.to_string(),
            OFFSET_MAX.to_string(),
            crate::api::protection::PARAMS_MAX.to_string(),
            crate::api::protection::QUERY_MAX.to_string(),
            GENERATIONS_MAX.to_string(),
            MODALITES_MAX.to_string(),
        ] {
            assert!(
                texte.contains(&attendu),
                "limite « {attendu} » absente de la description"
            );
        }
    }
}
