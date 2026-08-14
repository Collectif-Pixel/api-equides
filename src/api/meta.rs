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
    remarques: Vec<&'a str>,
}

pub async fn meta(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    json(&Meta {
        version_api: crate::VERSION,
        jeu_de_donnees: &etat.store.meta,
        remarques: vec![
            "Les identifiants exposés sont ceux de l'IFCE : ils figurent tels quels dans \
             l'URL de chaque fiche publique sur infochevaux.ifce.fr.",
            "La filiation est résolue en interne : 1 202 286 des 1 202 446 parents référencés \
             figurent au jeu de données. Les 160 restants sont des liens rompus, exposés comme \
             une absence de parent plutôt que comme une référence morte.",
            "« Sans performances » signifie qu'aucun indice n'est publié pour cet équidé, \
             non qu'il n'a jamais concouru : seuls 24,9 % des équidés en portent.",
            "Les millésimes de naissance aberrants présents dans la source sont conservés tels \
             quels ; cette API est un miroir fidèle de la donnée, pas un correcteur.",
            "Le champ `url` n'est pas stocké mais reconstruit depuis le slug et l'identifiant, \
             gabarit exposé par `url_modele`.",
            "Le champ `race` d'une fiche est pollué : il compte 11 747 valeurs distinctes, \
             dont une part sont des chronos de course (« 1'10\"2 (ASC6H) ») et non des races. \
             La race déclarée sur un lien de filiation, elle, n'en prend que 13, toutes propres — \
             c'est celle que porte le champ `race` des objets `pere`, `mere` et `pere_de_mere`. \
             Ce défaut vient de la source et n'est pas corrigé ici.",
            "API Équidés est un service indépendant, non affilié à l'IFCE. La donnée servie \
             est une extraction datée, dont l'empreinte figure ci-dessus ; elle peut diverger \
             des fiches en ligne, qui font seules autorité.",
            "Réutilisation : l'IFCE est un établissement public administratif (décret n° 2010-90). \
             L'article L321-1 du code des relations entre le public et l'administration ouvre la \
             réutilisation des informations publiques qu'il diffuse ; l'article L321-3 lui interdit \
             d'opposer le droit sui generis du producteur de base de données ; et le décret \
             n° 2016-1617, qui fixe limitativement les informations soumises à redevance, ne \
             mentionne ni l'IFCE ni les données équines.",
            "La réutilisation impose de mentionner la source et la date de mise à jour \
             (CRPA art. L322-1), et de ne pas dénaturer la donnée. Les champs `source`, \
             `ingere_le` et `empreinte_source` ci-dessus y pourvoient.",
            "Cette API n'expose aucune donnée à caractère personnel : elle ne sert que des \
             informations relatives à des animaux.",
        ],
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
