use crate::store::Store;
use serde::Serialize;

#[derive(Serialize)]
pub struct Reference<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub nom: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub race: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pays: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<&'a str>,
    pub fiche_disponible: bool,
}

impl<'a> Reference<'a> {
    pub fn depuis_lien(store: &'a Store, lien: &crate::store::Lien<'a>) -> Self {
        let id = lien.cible.map(|c| store.id(c));
        Self {
            fiche_disponible: id.is_some(),
            id,
            nom: lien.nom,
            race: lien.race,
            pays: lien.pays,
            alias: lien.alias,
        }
    }

    pub fn depuis(store: &'a Store, row: u32) -> Self {
        Self {
            id: Some(store.id(row)),
            nom: store.nom(row),
            race: store.race_libelle(row),
            pays: None,
            alias: None,
            fiche_disponible: true,
        }
    }
}

#[derive(Serialize)]
pub struct Produit<'a> {
    pub id: String,
    pub nom: &'a str,
    pub race: &'a str,
    pub sexe: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annee_naissance: Option<i32>,
    pub fiche_disponible: bool,
}

impl<'a> Produit<'a> {
    pub fn depuis(store: &'a Store, row: u32) -> Self {
        Self {
            id: store.id(row),
            nom: store.nom(row),
            race: store.race_libelle(row),
            sexe: store.sexe_libelle(row),
            annee_naissance: store.annee(row),
            fiche_disponible: true,
        }
    }
}

#[derive(Serialize)]
pub struct Indice<'a> {
    pub code: &'a str,
    pub valeur: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coefficient: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annee: Option<i32>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub appreciation: &'a str,
}

#[derive(Serialize)]
pub struct Performance<'a> {
    pub discipline: &'a str,
    pub texte: &'a str,
    pub indices: Vec<Indice<'a>>,
}

#[derive(Serialize)]
pub struct Equide<'a> {
    pub id: String,
    pub nom: &'a str,
    pub slug: &'a str,
    pub url: String,
    pub race: &'a str,
    pub sexe: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robe: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annee_naissance: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filiation_texte: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pere: Option<Reference<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mere: Option<Reference<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pere_de_mere: Option<Reference<'a>>,
    pub nombre_de_descendants: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub performances: Vec<Performance<'a>>,
}

fn non_vide(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

impl<'a> Equide<'a> {
    pub fn depuis(store: &'a Store, row: u32) -> Self {
        let id = store.id(row);
        let performances = store
            .performances(row)
            .map(|p| Performance {
                discipline: store.perf_discipline(p),
                texte: store.perf_texte(p),
                indices: store
                    .indices(p)
                    .map(|i| Indice {
                        code: store.indice_code(i),
                        valeur: store.indice_valeur(i),
                        coefficient: store.indice_coefficient(i),
                        annee: store.indice_annee(i),
                        appreciation: store.indice_appreciation(i),
                    })
                    .collect(),
            })
            .collect();

        Self {
            url: store.url(row),
            id,
            nom: store.nom(row),
            slug: store.slug(row),
            race: store.race_libelle(row),
            sexe: store.sexe_libelle(row),
            robe: non_vide(store.robe_libelle(row)),
            annee_naissance: store.annee(row),
            filiation_texte: non_vide(store.filiation_texte(row)),
            pere: store
                .lien(row, crate::store::PERE)
                .map(|l| Reference::depuis_lien(store, &l)),
            mere: store
                .lien(row, crate::store::MERE)
                .map(|l| Reference::depuis_lien(store, &l)),
            pere_de_mere: store
                .lien(row, crate::store::PERE_DE_MERE)
                .map(|l| Reference::depuis_lien(store, &l)),
            nombre_de_descendants: store.enfants(row).len(),
            performances,
        }
    }
}

#[derive(Serialize)]
pub struct Resume<'a> {
    pub id: String,
    pub nom: &'a str,
    pub race: &'a str,
    pub sexe: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robe: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annee_naissance: Option<i32>,
}

impl<'a> Resume<'a> {
    pub fn depuis(store: &'a Store, row: u32) -> Self {
        Self {
            id: store.id(row),
            nom: store.nom(row),
            race: store.race_libelle(row),
            sexe: store.sexe_libelle(row),
            robe: non_vide(store.robe_libelle(row)),
            annee_naissance: store.annee(row),
        }
    }
}

#[derive(Serialize)]
pub struct Noeud<'a> {
    pub id: String,
    pub nom: &'a str,
    pub race: &'a str,
    pub sexe: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annee_naissance: Option<i32>,
    pub generation: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pere: Option<Box<Noeud<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mere: Option<Box<Noeud<'a>>>,
}

impl<'a> Noeud<'a> {
    pub fn construire(store: &'a Store, row: u32, generation: u32, restant: u32) -> Self {
        let (pere, mere) = if restant == 0 {
            (None, None)
        } else {
            (
                store
                    .pere(row)
                    .map(|p| Box::new(Self::construire(store, p, generation + 1, restant - 1))),
                store
                    .mere(row)
                    .map(|m| Box::new(Self::construire(store, m, generation + 1, restant - 1))),
            )
        };
        Self {
            id: store.id(row),
            nom: store.nom(row),
            race: store.race_libelle(row),
            sexe: store.sexe_libelle(row),
            annee_naissance: store.annee(row),
            generation,
            pere,
            mere,
        }
    }
}

#[derive(Serialize)]
pub struct Page<T> {
    pub donnees: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Serialize)]
pub struct Pagination {
    pub total: u64,
    pub limite: usize,
    pub offset: usize,
    pub suivant: Option<String>,
    pub precedent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

#[derive(Serialize)]
pub struct Modalite<'a> {
    pub valeur: &'a str,
    pub nombre: u64,
}

#[derive(Serialize)]
pub struct Tranche {
    pub valeur: String,
    pub nombre: u64,
}
