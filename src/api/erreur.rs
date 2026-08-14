use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub struct Type {
    pub uri: &'static str,
    pub titre: &'static str,
    pub statut: StatusCode,
    pub quand: &'static str,
}

impl Type {
    pub fn ancre(&self) -> &'static str {
        self.uri
            .rsplit_once('#')
            .expect("un type porte toujours un fragment")
            .1
    }
}

pub static REQUETE_INVALIDE: Type = Type {
    uri: "https://docs.api-equides.org/guides/erreurs/#erreur-requete-invalide",
    titre: "Requête invalide",
    statut: StatusCode::BAD_REQUEST,
    quand: "La requête ne peut pas être traitée telle quelle, sans qu'un paramètre \
            précis soit en cause.",
};

pub static PARAMETRE_INVALIDE: Type = Type {
    uri: "https://docs.api-equides.org/guides/erreurs/#erreur-parametre-invalide",
    titre: "Paramètre invalide",
    statut: StatusCode::BAD_REQUEST,
    quand: "Un paramètre est inconnu, mal typé, hors bornes, ou porte une valeur \
            que le référentiel n'admet pas. Le champ `parametre` le nomme et \
            `indice` propose la correction — une faute de frappe reçoit la \
            suggestion du paramètre le plus proche.",
};

pub static INTROUVABLE: Type = Type {
    uri: "https://docs.api-equides.org/guides/erreurs/#erreur-introuvable",
    titre: "Ressource introuvable",
    statut: StatusCode::NOT_FOUND,
    quand: "L'identifiant ne correspond à aucune fiche du jeu de données, ou la \
            route n'existe pas. Un identifiant mal formé donne le même résultat \
            qu'un identifiant inconnu.",
};

pub static TROP_DE_REQUETES: Type = Type {
    uri: "https://docs.api-equides.org/guides/erreurs/#erreur-trop-de-requetes",
    titre: "Trop de requêtes",
    statut: StatusCode::TOO_MANY_REQUESTS,
    quand: "Le quota de débit de l'adresse appelante est dépassé. La réponse porte \
            un `Retry-After` indiquant le délai à respecter.",
};

pub static SERVICE_SURCHARGE: Type = Type {
    uri: "https://docs.api-equides.org/guides/erreurs/#erreur-service-surcharge",
    titre: "Service momentanément surchargé",
    statut: StatusCode::SERVICE_UNAVAILABLE,
    quand: "La capacité de traitement simultané est atteinte, ou la requête a \
            dépassé le délai maximal. Le service refuse tôt, avec un \
            `Retry-After`, plutôt que de faire attendre une réponse qui \
            expirerait.",
};

pub static CATALOGUE: &[&Type] = &[
    &PARAMETRE_INVALIDE,
    &REQUETE_INVALIDE,
    &INTROUVABLE,
    &TROP_DE_REQUETES,
    &SERVICE_SURCHARGE,
];

#[derive(Debug, Serialize)]
pub struct Probleme {
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub title: &'static str,
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parametre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indice: Option<String>,
    pub documentation: &'static str,
}

#[derive(Debug)]
pub struct Erreur {
    pub statut: StatusCode,
    pub corps: Box<Probleme>,
}

impl Erreur {
    fn nouvelle(type_: &'static Type, detail: impl Into<String>) -> Self {
        Self {
            statut: type_.statut,
            corps: Box::new(Probleme {
                type_: type_.uri,
                title: type_.titre,
                status: type_.statut.as_u16(),
                detail: detail.into(),
                parametre: None,
                indice: None,
                documentation: "/",
            }),
        }
    }

    pub fn requete_invalide(detail: impl Into<String>) -> Self {
        Self::nouvelle(&REQUETE_INVALIDE, detail)
    }

    pub fn parametre_invalide(
        parametre: impl Into<String>,
        detail: impl Into<String>,
        indice: impl Into<String>,
    ) -> Self {
        let mut e = Self::nouvelle(&PARAMETRE_INVALIDE, detail);
        e.corps.parametre = Some(parametre.into());
        e.corps.indice = Some(indice.into());
        e
    }

    pub fn introuvable(detail: impl Into<String>) -> Self {
        Self::nouvelle(&INTROUVABLE, detail)
    }

    pub fn service_surcharge(detail: impl Into<String>) -> Self {
        let mut e = Self::nouvelle(&SERVICE_SURCHARGE, detail);
        e.corps.indice = Some(
            "réessayez dans quelques instants ; les réponses portent un ETag, \
             une revalidation conditionnelle coûte beaucoup moins cher"
                .to_string(),
        );
        e
    }

    pub fn trop_de_requetes(detail: impl Into<String>) -> Self {
        let mut e = Self::nouvelle(&TROP_DE_REQUETES, detail);
        e.corps.indice = Some(
            "cette API est publique et non authentifiée ; les limites de capacité \
             sont documentées à la racine du service"
                .to_string(),
        );
        e
    }
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} : {}", self.corps.title, self.corps.detail)
    }
}

impl std::error::Error for Erreur {}

impl IntoResponse for Erreur {
    fn into_response(self) -> Response {
        let corps = serde_json::to_vec(&*self.corps)
            .unwrap_or_else(|_| br#"{"title":"Erreur interne","status":500}"#.to_vec());
        (
            self.statut,
            [(
                header::CONTENT_TYPE,
                "application/problem+json; charset=utf-8",
            )],
            corps,
        )
            .into_response()
    }
}

impl From<crate::query::ErreurFiltre> for Erreur {
    fn from(e: crate::query::ErreurFiltre) -> Self {
        Erreur::parametre_invalide(e.parametre, e.to_string(), e.indice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_corps_porte_les_champs_normalises() {
        let e = Erreur::parametre_invalide("race", "valeur inconnue", "voir le référentiel");
        let v = serde_json::to_value(&e.corps).unwrap();
        for champ in ["type", "title", "status", "detail"] {
            assert!(v.get(champ).is_some(), "champ RFC 9457 manquant : {champ}");
        }
        assert_eq!(v["status"], 400);
        assert_eq!(v["parametre"], "race");
        assert_eq!(v["indice"], "voir le référentiel");
    }

    #[test]
    fn les_champs_optionnels_sont_omis_et_non_nuls() {
        let e = Erreur::introuvable("rien ici");
        let v = serde_json::to_value(&e.corps).unwrap();
        assert!(v.get("parametre").is_none());
        assert!(v.get("indice").is_none());
    }

    #[test]
    fn chaque_type_a_son_uri_et_son_statut() {
        let cas = [
            (Erreur::requete_invalide("x"), 400),
            (Erreur::parametre_invalide("p", "x", "y"), 400),
            (Erreur::introuvable("x"), 404),
            (Erreur::trop_de_requetes("x"), 429),
            (Erreur::service_surcharge("x"), 503),
        ];
        let mut types = Vec::new();
        for (e, statut) in cas {
            assert_eq!(e.corps.status, statut);
            assert_eq!(e.statut.as_u16(), statut);
            assert!(
                e.corps
                    .type_
                    .starts_with("https://docs.api-equides.org/guides/erreurs/#erreur-")
            );
            types.push(e.corps.type_);
        }
        types.sort_unstable();
        let avant = types.len();
        types.dedup();
        assert_eq!(
            types.len(),
            avant,
            "chaque type de problème doit avoir son URI"
        );
    }

    #[test]
    fn tout_type_produit_figure_au_catalogue() {
        let produits = [
            Erreur::requete_invalide("x"),
            Erreur::parametre_invalide("p", "x", "y"),
            Erreur::introuvable("x"),
            Erreur::trop_de_requetes("x"),
            Erreur::service_surcharge("x"),
        ];
        for e in &produits {
            assert!(
                CATALOGUE.iter().any(|t| t.uri == e.corps.type_),
                "type absent du catalogue : {}",
                e.corps.type_
            );
        }
        assert_eq!(
            CATALOGUE.len(),
            produits.len(),
            "le catalogue annonce un type que plus aucun constructeur ne produit"
        );
    }

    #[test]
    fn l_ancre_est_le_fragment_de_l_uri() {
        for t in CATALOGUE {
            assert!(t.uri.ends_with(&format!("#{}", t.ancre())));
            assert!(!t.quand.is_empty(), "{} sans description", t.uri);
        }
    }
}
