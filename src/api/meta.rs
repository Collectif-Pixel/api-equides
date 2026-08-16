use super::EtatPartage;
use super::equides::json;
use super::erreur::Erreur;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub async fn racine(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    json(&serde_json::json!({
        "nom": "API Équidés",
        "version": crate::VERSION,
        "description": "API REST ouverte sur les fiches publiques d'équidés de l'IFCE, \
                        avec généalogie et indices de performance.",
        "affiliation": "Service indépendant, non affilié à l'IFCE.",
        "lignes": etat.store.n,
        "documentation": "https://docs.api-equides.org",
        "openapi": "/openapi.json",
        "endpoints": {
            "recherche": "/v1/equides",
            "autocompletion": "/v1/search?q=",
            "consultation": "/v1/equides/{id}",
            "pedigree": "/v1/equides/{id}/pedigree",
            "descendance": "/v1/equides/{id}/descendance",
            "referentiels": "/v1/referentiels/{races|robes|sexes|disciplines|codes_indice|races_lien|annees_naissance}",
            "statistiques": "/v1/stats",
            "repartitions": "/v1/stats/repartition?dimension=race",
            "metadonnees": "/v1/meta",
            "sante": "/healthz"
        },
        "gabarits_url": {
            "fiche": "/v1/equides/{id}",
            "pedigree": "/v1/equides/{id}/pedigree",
            "descendance": "/v1/equides/{id}/descendance",
            "fiche_source": etat.store.meta.url_modele,
        },
        "licence_donnees": etat.store.meta.licence,
        "source": etat.store.meta.source,
    }))
}

#[derive(Serialize)]
struct Meta<'a> {
    version_api: &'a str,
    jeu_de_donnees: &'a crate::snapshot::DatasetMeta,
    remarques: Vec<String>,
}

fn milliers(n: u64) -> String {
    let chiffres = n.to_string();
    let mut out = String::with_capacity(chiffres.len() + chiffres.len() / 3);
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn pourcentage(partie: u64, total: u64) -> String {
    if total == 0 {
        return "0,0".to_string();
    }
    format!("{:.1}", partie as f64 * 100.0 / total as f64).replace('.', ",")
}

fn remarques(store: &crate::store::Store) -> Vec<String> {
    let a = &store.meta.anomalies;
    let total = u64::from(store.n);
    let perfs = store.bitmap_avec_performances().len();
    let records = store.bitmap_avec_record().len();
    let statuts = store.nombre_avec_statut_reproducteur();
    let resolus = a.parents_references.saturating_sub(a.parents_pendants);

    vec![
        "Les identifiants exposés sont ceux de l'IFCE : ils figurent tels quels dans \
         l'URL de chaque fiche publique sur infochevaux.ifce.fr."
            .to_string(),
        format!(
            "La filiation est résolue en interne : {resolus} des {references} parents distincts \
             cités figurent au jeu de données. Les {pendants} autres sont des liens rompus, \
             exposés comme une absence de parent plutôt que comme une référence morte. \
             Ces deux nombres dénombrent des identifiants ; les citations, elles, sont plus \
             nombreuses puisqu'un même parent est cité par plusieurs fiches : \
             {citations_pendantes} citations rompues sur {citations} emplacements `pere`, \
             `mere` ou `pere_de_mere` renseignés. `anomalies` publie les quatre.",
            resolus = milliers(u64::from(resolus)),
            references = milliers(u64::from(a.parents_references)),
            pendants = milliers(u64::from(a.parents_pendants)),
            citations_pendantes = milliers(u64::from(a.references_pendantes)),
            citations = milliers(u64::from(a.references_de_parents)),
        ),
        format!(
            "« Sans performances » signifie qu'aucun indice n'est publié pour cet équidé, \
             non qu'il n'a jamais concouru : seuls {} % des équidés en portent.",
            pourcentage(perfs, total)
        ),
        "Les millésimes de naissance aberrants présents dans la source sont conservés tels \
         quels ; cette API est un miroir fidèle de la donnée, pas un correcteur."
            .to_string(),
        "Le champ `url` n'est pas stocké mais reconstruit depuis le slug et l'identifiant, \
         gabarit exposé par `url_modele`."
            .to_string(),
        format!(
            "Deux nomenclatures de race coexistent, et ne se recouvrent pas. Le champ `race` \
             d'une fiche porte le libellé développé de la source ({races} valeurs, « Trotteur \
             Francais ») ; celui des objets `pere`, `mere` et `pere_de_mere` porte le code \
             abrégé du lien de filiation ({races_lien} valeurs, « TF »). Un filtre `race=` \
             attend la première forme : /v1/referentiels/races et \
             /v1/referentiels/races_lien listent l'une et l'autre.",
            races = milliers(store.races.len() as u64),
            races_lien = milliers(store.races_lien.len() as u64),
        ),
        format!(
            "`record` porte le chrono de course déclaré par la source, sur {records} fiches \
             ({pct_records} % du jeu) et sous la forme « 1'16\"7 (AEL3V) ». C'est une donnée \
             textuelle, ni comparable ni triable en l'état.",
            records = milliers(records),
            pct_records = pourcentage(records, total),
        ),
        format!(
            "`statut_reproducteur` est déclaré par la source ({statuts} fiches, \
             {pct_statuts} % du jeu), là où `nombre_de_descendants` est calculé par cette API. \
             Les deux ne se déduisent pas l'un de l'autre : une jument portant « Pouliniere » \
             sans aucun produit enregistré n'est pas une jument jamais mise à la reproduction. \
             L'absence de statut est un silence de la source, pas une négation. \
             Modalités sur /v1/referentiels/statuts_reproducteur.",
            statuts = milliers(statuts),
            pct_statuts = pourcentage(statuts, total),
        ),
        "API Équidés est un service indépendant, non affilié à l'IFCE. La donnée servie \
         est une extraction datée, dont l'empreinte figure ci-dessus ; elle peut diverger \
         des fiches en ligne, qui font seules autorité."
            .to_string(),
        "Réutilisation : l'IFCE est un établissement public administratif (décret n° 2010-90). \
         L'article L321-1 du code des relations entre le public et l'administration ouvre la \
         réutilisation des informations publiques qu'il diffuse ; l'article L321-3 lui interdit \
         d'opposer le droit sui generis du producteur de base de données ; et le décret \
         n° 2016-1617, qui fixe limitativement les informations soumises à redevance, ne \
         mentionne ni l'IFCE ni les données équines."
            .to_string(),
        format!(
            "La réutilisation impose de mentionner la source et la date de mise à jour \
             (CRPA art. L322-1), et de ne pas dénaturer la donnée. Les champs `source` et \
             `ingere_le` y pourvoient. `empreinte_source` désigne l'extraction sans ambiguïté : \
             c'est l'empreinte {algo}, annoncée par `algorithme_empreinte`, du fichier \
             `{fichier}` tel qu'ingéré — octet pour octet, {octets} octets, et non un export \
             normalisé. Hacher le fichier publié doit redonner exactement cette valeur.",
            algo = store.meta.algorithme_empreinte,
            fichier = store.meta.fichier_source,
            octets = milliers(store.meta.octets_source),
        ),
        "Cette API n'expose aucune donnée à caractère personnel : elle ne sert que des \
         informations relatives à des animaux. Le champ `naisseur` de la source est écarté \
         dès l'ingestion, et n'existe dans aucune image."
            .to_string(),
    ]
}

pub async fn meta(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    json(&Meta {
        version_api: crate::VERSION,
        jeu_de_donnees: &etat.store.meta,
        remarques: remarques(&etat.store),
    })
}

pub async fn sante(State(etat): State<EtatPartage>) -> Response {
    let pret = etat.store.n > 0;
    let corps = serde_json::json!({
        "statut": if pret { "ok" } else { "indisponible" },
        "lignes": etat.store.n,
        "version": crate::VERSION,
    });
    let statut = if pret {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (statut, axum::Json(corps)).into_response()
}

pub async fn metriques(State(etat): State<EtatPartage>) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        etat.metrics.rendre(etat.store.n),
    )
        .into_response()
}

pub async fn openapi(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    json(&crate::openapi::document(&etat.store))
}

pub async fn route_inconnue(uri: axum::http::Uri) -> Erreur {
    Erreur::introuvable(format!("aucune route ne correspond à « {} »", uri.path()))
}
