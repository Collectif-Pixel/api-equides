use super::EtatPartage;
use super::equides::json;
use super::erreur::Erreur;
use crate::model::{Modalite, Tranche};
use crate::store::Dict;
use axum::extract::{Path, State};
use axum::response::Response;
use roaring::RoaringBitmap;
use serde::Serialize;

#[derive(Serialize)]
struct Reponse<'a, T> {
    dimension: &'a str,
    nombre_de_modalites: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    donnees: Vec<T>,
}

pub const MODALITES_MAX: usize = 2_000;

fn modalites<'a>(
    dict: &'a Dict,
    bitmap: impl Fn(u32) -> Option<&'a RoaringBitmap>,
) -> Vec<Modalite<'a>> {
    let mut v: Vec<Modalite<'a>> = dict
        .iter()
        .map(|(code, valeur)| Modalite {
            valeur,
            nombre: bitmap(code).map_or(0, roaring::RoaringBitmap::len),
        })
        .collect();
    v.sort_unstable_by(|a, b| b.nombre.cmp(&a.nombre).then(a.valeur.cmp(b.valeur)));
    v
}

pub async fn lister(
    State(etat): State<EtatPartage>,
    Path(dimension): Path<String>,
) -> Result<Response, Erreur> {
    let store = &etat.store;

    if dimension == "annees_naissance" {
        let donnees: Vec<Tranche> = store
            .annees()
            .map(|(a, n)| Tranche {
                valeur: a.to_string(),
                nombre: n,
            })
            .collect();
        return json(&Reponse {
            dimension: &dimension,
            nombre_de_modalites: donnees.len(),
            note: None,
            donnees,
        });
    }

    let toutes = match dimension.as_str() {
        "races" => modalites(&store.races, |c| store.bitmap_race(c)),
        "robes" => modalites(&store.robes, |c| store.bitmap_robe(c)),
        "sexes" => modalites(&store.sexes, |c| store.bitmap_sexe(c)),
        "disciplines" => modalites(&store.disciplines, |c| store.bitmap_discipline(c)),
        "codes_indice" => modalites(&store.codes_indice, |c| store.bitmap_code_indice(c)),
        "races_lien" => modalites(&store.races_lien, |_| None),
        autre => {
            return Err(Erreur::parametre_invalide(
                "dimension",
                format!("référentiel inconnu « {autre} »"),
                "référentiels disponibles : races, robes, sexes, disciplines, \
                 codes_indice, races_lien, annees_naissance",
            ));
        }
    };

    let total = toutes.len();
    let tronque = total > MODALITES_MAX;
    let donnees: Vec<Modalite> = toutes.into_iter().take(MODALITES_MAX).collect();

    json(&Reponse {
        dimension: &dimension,
        nombre_de_modalites: total,
        note: tronque.then(|| {
            format!("liste tronquée aux {MODALITES_MAX} modalités les plus fréquentes sur {total}")
        }),
        donnees,
    })
}
