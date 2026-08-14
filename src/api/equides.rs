use super::EtatPartage;
use super::erreur::Erreur;
use super::params::{PARAMS_FILTRE, Params};
use crate::model::{Equide, Noeud, Page, Pagination, Reference, Resume};
use crate::query::{self, GENERATIONS_MAX, LIMITE_MAX, OFFSET_MAX};
use axum::extract::{Path, State};
use axum::http::{Uri, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub fn json(valeur: &impl Serialize) -> Result<Response, Erreur> {
    let corps = serde_json::to_vec(valeur).map_err(|e| {
        tracing::error!(erreur = %e, "échec de sérialisation JSON");
        Erreur::requete_invalide("réponse non sérialisable")
    })?;
    Ok((
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        corps,
    )
        .into_response())
}

pub const SUGGESTIONS_DEFAUT: usize = 10;

pub const DESCENDANCE_DEFAUT: usize = 50;

pub const GENERATIONS_DEFAUT: u32 = 4;

fn params_admis() -> Vec<&'static str> {
    let mut v = PARAMS_FILTRE.to_vec();
    v.extend_from_slice(&["tri", "limite", "offset", "vue"]);
    v
}

pub async fn lister(State(etat): State<EtatPartage>, uri: Uri) -> Result<Response, Erreur> {
    let p = Params::analyser(uri.query())?;
    p.refuser_inconnus(&params_admis())?;

    let filtres = p.filtres()?;
    let (tri, sens) = p.tri()?;
    let (limite, offset) = p.pagination()?;

    let complet = match p.premier("vue") {
        None | Some("resume") => false,
        Some("complet") => true,
        Some(autre) => {
            return Err(Erreur::parametre_invalide(
                "vue",
                format!("vue inconnue « {autre} »"),
                "valeurs admises : resume (défaut), complet",
            ));
        }
    };

    let store = &etat.store;
    let selection = query::resoudre(store, &filtres)?;
    let total = selection.len();
    let lignes = query::paginer(store, &selection, tri, sens, offset, limite);

    let pagination = Pagination {
        total,
        limite,
        offset,
        suivant: lien_page(&uri, offset.saturating_add(limite), limite, total),
        precedent: offset
            .checked_sub(limite)
            .and_then(|o| lien_page(&uri, o, limite, total)),
        note: (total > OFFSET_MAX as u64).then_some(
            "au-delà de 10 000 résultats, la pagination s'arrête : affinez les filtres, \
             ou téléchargez le fichier publié dont /v1/meta donne la source",
        ),
    };

    if complet {
        json(&Page {
            donnees: lignes
                .iter()
                .map(|&row| Equide::depuis(store, row))
                .collect::<Vec<_>>(),
            pagination,
        })
    } else {
        json(&Page {
            donnees: lignes
                .iter()
                .map(|&row| Resume::depuis(store, row))
                .collect::<Vec<_>>(),
            pagination,
        })
    }
}

fn lien_page(uri: &Uri, offset: usize, limite: usize, total: u64) -> Option<String> {
    if offset as u64 >= total || offset > OFFSET_MAX {
        return None;
    }
    let mut enc = form_urlencoded::Serializer::new(String::new());
    for (k, v) in form_urlencoded::parse(uri.query().unwrap_or("").as_bytes()) {
        if k != "offset" && k != "limite" {
            enc.append_pair(&k, &v);
        }
    }
    enc.append_pair("limite", &limite.to_string());
    enc.append_pair("offset", &offset.to_string());
    Some(format!("{}?{}", uri.path(), enc.finish()))
}

fn ligne_ou_404(etat: &EtatPartage, id: &str) -> Result<u32, Erreur> {
    etat.store
        .row_by_id(id)
        .ok_or_else(|| Erreur::introuvable(format!("aucun équidé ne porte l'identifiant « {id} »")))
}

#[derive(Serialize)]
struct Suggestion<'a> {
    id: String,
    nom: &'a str,
    race: &'a str,
    sexe: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    annee_naissance: Option<i32>,
    score: f32,
}

#[derive(Serialize)]
struct Recherche<'a> {
    q: &'a str,
    limite: usize,
    resultats: Vec<Suggestion<'a>>,
}

pub async fn rechercher(State(etat): State<EtatPartage>, uri: Uri) -> Result<Response, Erreur> {
    let p = Params::analyser(uri.query())?;
    p.refuser_inconnus(&["q", "limite"])?;

    let q = p.premier("q").unwrap_or("");
    if q.is_empty() {
        return Err(Erreur::parametre_invalide(
            "q",
            "le paramètre `q` est obligatoire",
            "exemple : /v1/search?q=qabalah",
        ));
    }
    let limite = p
        .entier("limite")?
        .unwrap_or(SUGGESTIONS_DEFAUT)
        .clamp(1, LIMITE_MAX);

    let store = &etat.store;
    let resultats = store
        .noms_index
        .suggerer(q, limite)
        .into_iter()
        .map(|(row, score)| Suggestion {
            id: store.id(row),
            nom: store.nom(row),
            race: store.race_libelle(row),
            sexe: store.sexe_libelle(row),
            annee_naissance: store.annee(row),
            score,
        })
        .collect();

    json(&Recherche {
        q,
        limite,
        resultats,
    })
}

pub async fn par_id(
    State(etat): State<EtatPartage>,
    Path(id): Path<String>,
) -> Result<Response, Erreur> {
    let row = ligne_ou_404(&etat, &id)?;
    json(&Equide::depuis(&etat.store, row))
}

#[derive(Serialize)]
struct Pedigree<'a> {
    generations: u32,
    noeuds: usize,
    noeuds_theoriques: u32,
    arbre: Noeud<'a>,
}

fn compter(n: &Noeud<'_>) -> usize {
    1 + n.pere.as_deref().map_or(0, compter) + n.mere.as_deref().map_or(0, compter)
}

pub async fn pedigree(
    State(etat): State<EtatPartage>,
    Path(id): Path<String>,
    uri: Uri,
) -> Result<Response, Erreur> {
    let p = Params::analyser(uri.query())?;
    p.refuser_inconnus(&["generations"])?;
    let generations = p
        .entier("generations")?
        .unwrap_or(GENERATIONS_DEFAUT as usize);
    if generations > GENERATIONS_MAX as usize {
        return Err(Erreur::parametre_invalide(
            "generations",
            format!("profondeur plafonnée à {GENERATIONS_MAX}, reçu {generations}"),
            "un pedigree double de taille à chaque génération : 10 générations \
             représentent déjà jusqu'à 1 023 ancêtres",
        ));
    }

    let generations = generations as u32;
    let row = ligne_ou_404(&etat, &id)?;
    let arbre = Noeud::construire(&etat.store, row, 0, generations);
    json(&Pedigree {
        generations,
        noeuds: compter(&arbre),
        noeuds_theoriques: 2u32.saturating_pow(generations + 1) - 1,
        arbre,
    })
}

#[derive(Serialize)]
struct Descendance<'a> {
    parent: Reference<'a>,
    total: usize,
    limite: usize,
    offset: usize,
    donnees: Vec<Reference<'a>>,
}

pub async fn descendance(
    State(etat): State<EtatPartage>,
    Path(id): Path<String>,
    uri: Uri,
) -> Result<Response, Erreur> {
    let p = Params::analyser(uri.query())?;
    p.refuser_inconnus(&["limite", "offset"])?;
    let limite = p
        .entier("limite")?
        .unwrap_or(DESCENDANCE_DEFAUT)
        .clamp(1, LIMITE_MAX);
    let offset = p.entier("offset")?.unwrap_or(0);

    let row = ligne_ou_404(&etat, &id)?;
    let store = &etat.store;
    let enfants = store.enfants(row);

    json(&Descendance {
        parent: Reference::depuis(store, row),
        total: enfants.len(),
        limite,
        offset,
        donnees: enfants
            .iter()
            .skip(offset)
            .take(limite)
            .map(|&r| Reference::depuis(store, r))
            .collect(),
    })
}
