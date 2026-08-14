use super::EtatPartage;
use super::equides::json;
use super::erreur::Erreur;
use super::params::{PARAMS_FILTRE, Params};
use crate::model::Tranche;
use crate::query;
use crate::store::Dict;
use axum::extract::State;
use axum::http::Uri;
use axum::response::Response;
use roaring::RoaringBitmap;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Serialize)]
struct Global<'a> {
    total: u32,
    avec_performances: u64,
    sans_performances: u64,
    avec_filiation: u64,
    sans_filiation: u64,
    par_sexe: Vec<Tranche>,
    nombre_de_races: usize,
    nombre_de_robes: usize,
    nombre_de_disciplines: usize,
    annee_naissance_min: Option<i32>,
    annee_naissance_max: Option<i32>,
    avertissement: &'a str,
}

pub async fn global(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    let s = &etat.store;
    let total = s.n;
    let perfs = s.bitmap_avec_performances().len();
    let sans_filiation = s.meta.anomalies.sans_filiation as u64;

    let mut par_sexe: Vec<Tranche> = s
        .sexes
        .iter()
        .map(|(code, valeur)| Tranche {
            valeur: valeur.to_string(),
            nombre: s.bitmap_sexe(code).map_or(0, roaring::RoaringBitmap::len),
        })
        .collect();
    par_sexe.sort_unstable_by_key(|t| std::cmp::Reverse(t.nombre));

    json(&Global {
        total,
        avec_performances: perfs,
        sans_performances: total as u64 - perfs,
        avec_filiation: total as u64 - sans_filiation,
        sans_filiation,
        par_sexe,
        nombre_de_races: s.races.len(),
        nombre_de_robes: s.robes.len(),
        nombre_de_disciplines: s.disciplines.len(),
        annee_naissance_min: s.meta.anomalies.annee_naissance_min,
        annee_naissance_max: s.meta.anomalies.annee_naissance_max,
        avertissement: "« sans performances » signifie qu'aucun indice n'est publié pour cet \
                        équidé, non qu'il n'a jamais concouru.",
    })
}

fn ventiler_dimension<'a>(
    selection: &RoaringBitmap,
    dict: &'a Dict,
    bitmap: impl Fn(u32) -> Option<&'a RoaringBitmap>,
    limite: usize,
) -> Vec<Tranche> {
    let mut tas: BinaryHeap<Reverse<(u64, &str)>> = BinaryHeap::with_capacity(limite + 1);
    for (code, valeur) in dict.iter() {
        let nombre = bitmap(code).map_or(0, |b| selection.intersection_len(b));
        if nombre == 0 {
            continue;
        }
        if tas.len() < limite {
            tas.push(Reverse((nombre, valeur)));
        } else if let Some(Reverse(sommet)) = tas.peek()
            && (nombre, valeur) > *sommet
        {
            tas.pop();
            tas.push(Reverse((nombre, valeur)));
        }
    }
    let mut retenues: Vec<(u64, &str)> = tas.into_iter().map(|Reverse(t)| t).collect();
    retenues.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    retenues
        .into_iter()
        .map(|(nombre, valeur)| Tranche {
            valeur: valeur.to_string(),
            nombre,
        })
        .collect()
}

pub const MODALITES_REPARTITION_MAX: usize = 1_000;

pub const MODALITES_REPARTITION_DEFAUT: usize = 50;

#[derive(Serialize)]
struct Repartition<'a> {
    dimension: &'a str,
    effectif_filtre: u64,
    effectif_affiche: u64,
    donnees: Vec<Tranche>,
}

pub async fn repartition(State(etat): State<EtatPartage>, uri: Uri) -> Result<Response, Erreur> {
    let p = Params::analyser(uri.query())?;
    let mut admis = PARAMS_FILTRE.to_vec();
    admis.extend_from_slice(&["dimension", "limite"]);
    p.refuser_inconnus(&admis)?;

    let dimension = p.premier("dimension").unwrap_or("race").to_string();
    let limite = p
        .entier("limite")?
        .unwrap_or(MODALITES_REPARTITION_DEFAUT)
        .clamp(1, MODALITES_REPARTITION_MAX);

    let store = &etat.store;
    let selection = query::resoudre(store, &p.filtres()?)?;
    let effectif_filtre = selection.len();

    let mut donnees: Vec<Tranche> = match dimension.as_str() {
        "race" => ventiler_dimension(&selection, &store.races, |c| store.bitmap_race(c), limite),
        "robe" => ventiler_dimension(&selection, &store.robes, |c| store.bitmap_robe(c), limite),
        "sexe" => ventiler_dimension(&selection, &store.sexes, |c| store.bitmap_sexe(c), limite),
        "discipline" => ventiler_dimension(
            &selection,
            &store.disciplines,
            |c| store.bitmap_discipline(c),
            limite,
        ),
        "annee_naissance" => store
            .bitmaps_annees()
            .map(|(annee, bm)| Tranche {
                valeur: annee.to_string(),
                nombre: selection.intersection_len(bm),
            })
            .collect(),
        autre => {
            return Err(Erreur::parametre_invalide(
                "dimension",
                format!("dimension inconnue « {autre} »"),
                "dimensions disponibles : race, robe, sexe, discipline, annee_naissance",
            ));
        }
    };

    if dimension == "annee_naissance" {
        donnees.retain(|t| t.nombre > 0);
        donnees.sort_unstable_by(|a, b| a.valeur.cmp(&b.valeur));
    }

    json(&Repartition {
        dimension: &dimension,
        effectif_filtre,
        effectif_affiche: donnees.iter().map(|t| t.nombre).sum(),
        donnees,
    })
}
