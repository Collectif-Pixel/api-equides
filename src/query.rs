use crate::store::Store;
use roaring::RoaringBitmap;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const LIMITE_MAX: usize = 100;
pub const LIMITE_DEFAUT: usize = 20;
pub const OFFSET_MAX: usize = 10_000;
pub const GENERATIONS_MAX: u32 = 8;

#[derive(Debug, Clone)]
pub struct ErreurFiltre {
    pub parametre: &'static str,
    pub valeur: String,
    pub indice: String,
}

impl std::fmt::Display for ErreurFiltre {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "valeur inconnue « {} » pour le paramètre `{}` ({})",
            self.valeur, self.parametre, self.indice
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct Filtres {
    pub races: Vec<String>,
    pub robes: Vec<String>,
    pub sexes: Vec<String>,
    pub disciplines: Vec<String>,
    pub codes_indice: Vec<String>,
    pub annees_naissance: Vec<i32>,
    pub nom: Option<String>,
    pub annee_min: Option<i32>,
    pub annee_max: Option<i32>,
    pub avec_performances: Option<bool>,
}

impl Filtres {
    pub fn est_vide(&self) -> bool {
        self.races.is_empty()
            && self.robes.is_empty()
            && self.sexes.is_empty()
            && self.disciplines.is_empty()
            && self.codes_indice.is_empty()
            && self.annees_naissance.is_empty()
            && self.nom.is_none()
            && self.annee_min.is_none()
            && self.annee_max.is_none()
            && self.avec_performances.is_none()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tri {
    #[default]
    Naturel,
    Nom,
    Annee,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Sens {
    #[default]
    Croissant,
    Decroissant,
}

impl Tri {
    pub fn parse(s: &str) -> Option<(Tri, Sens)> {
        let (sens, nom) = match s.strip_prefix('-') {
            Some(reste) => (Sens::Decroissant, reste),
            None => (Sens::Croissant, s),
        };
        let tri = match nom {
            "naturel" | "" => Tri::Naturel,
            "nom" => Tri::Nom,
            "annee" | "annee_naissance" => Tri::Annee,
            _ => return None,
        };
        Some((tri, sens))
    }
}

pub fn resoudre<'a>(
    store: &'a Store,
    filtres: &Filtres,
) -> Result<Cow<'a, RoaringBitmap>, ErreurFiltre> {
    if filtres.est_vide() {
        return Ok(Cow::Borrowed(store.toutes_les_lignes()));
    }

    let mut acc: Option<RoaringBitmap> = None;

    appliquer_dimension(&mut acc, &filtres.races, "race", "races", |v| {
        store.races.code(v).and_then(|c| store.bitmap_race(c))
    })?;
    appliquer_dimension(&mut acc, &filtres.robes, "robe", "robes", |v| {
        store.robes.code(v).and_then(|c| store.bitmap_robe(c))
    })?;
    appliquer_dimension(&mut acc, &filtres.sexes, "sexe", "sexes", |v| {
        store.sexes.code(v).and_then(|c| store.bitmap_sexe(c))
    })?;
    appliquer_dimension(
        &mut acc,
        &filtres.disciplines,
        "discipline",
        "disciplines",
        |v| {
            store
                .disciplines
                .code(v)
                .and_then(|c| store.bitmap_discipline(c))
        },
    )?;

    appliquer_dimension(
        &mut acc,
        &filtres.codes_indice,
        "indice",
        "codes_indice",
        |v| {
            store
                .codes_indice
                .code(v)
                .and_then(|c| store.bitmap_code_indice(c))
        },
    )?;

    if !filtres.annees_naissance.is_empty() {
        let mut union = RoaringBitmap::new();
        for a in &filtres.annees_naissance {
            if let Some(bm) = store.bitmap_annee(*a) {
                union |= bm;
            }
        }
        acc = Some(intersecter(acc, union));
    }

    if filtres.annee_min.is_some() || filtres.annee_max.is_some() {
        let mut union = RoaringBitmap::new();
        for (annee, bm) in store.bitmaps_annees() {
            let trop_tot = filtres.annee_min.is_some_and(|m| annee < m);
            let trop_tard = filtres.annee_max.is_some_and(|m| annee > m);
            if !trop_tot && !trop_tard {
                union |= bm;
            }
        }
        acc = Some(intersecter(acc, union));
    }

    if let Some(q) = &filtres.nom
        && let Some(bm) = store.noms_index.search(q)
    {
        acc = Some(intersecter(acc, bm));
    }

    if let Some(attendu) = filtres.avec_performances {
        let bm = store.bitmap_avec_performances();
        acc = Some(match (acc.take(), attendu) {
            (None, true) => bm.clone(),
            (None, false) => store.toutes_les_lignes() - bm,
            (Some(a), true) => a & bm,
            (Some(a), false) => a - bm,
        });
    }

    Ok(match acc {
        None => Cow::Borrowed(store.toutes_les_lignes()),
        Some(bm) => Cow::Owned(bm),
    })
}

fn intersecter(acc: Option<RoaringBitmap>, union: RoaringBitmap) -> RoaringBitmap {
    match acc {
        None => union,
        Some(a) => a & union,
    }
}

fn appliquer_dimension<'a>(
    acc: &mut Option<RoaringBitmap>,
    valeurs: &[String],
    parametre: &'static str,
    referentiel: &str,
    resoudre_valeur: impl Fn(&str) -> Option<&'a RoaringBitmap>,
) -> Result<(), ErreurFiltre> {
    if valeurs.is_empty() {
        return Ok(());
    }
    let mut union = RoaringBitmap::new();
    for v in valeurs {
        let bm = resoudre_valeur(v).ok_or_else(|| ErreurFiltre {
            parametre,
            valeur: v.clone(),
            indice: format!("valeurs admises sur /v1/referentiels/{referentiel}"),
        })?;
        union |= bm;
    }
    *acc = Some(intersecter(acc.take(), union));
    Ok(())
}

pub fn paginer(
    store: &Store,
    lignes: &RoaringBitmap,
    tri: Tri,
    sens: Sens,
    offset: usize,
    limite: usize,
) -> Vec<u32> {
    if tri == Tri::Naturel {
        return match sens {
            Sens::Croissant => lignes.iter().skip(offset).take(limite).collect(),
            Sens::Decroissant => {
                let total = lignes.len() as usize;
                let debut = total.saturating_sub(offset + limite);
                let nb = total.saturating_sub(offset).saturating_sub(debut);
                let mut page: Vec<u32> = lignes.iter().skip(debut).take(nb).collect();
                page.reverse();
                page
            }
        };
    }

    let k = offset.saturating_add(limite);
    let permutation = match tri {
        Tri::Nom => store.permutation_nom(),
        Tri::Annee => store.permutation_annee(),
        Tri::Naturel => unreachable!("traité plus haut"),
    };

    let taille = lignes.len() as usize;
    let cout_permutation = (store.n as usize)
        .checked_div(taille)
        .map_or(usize::MAX, |c| c.saturating_mul(k));
    if cout_permutation <= taille {
        let tout = taille == store.n as usize;
        return parcourir(permutation, lignes, tout, sens, offset, limite);
    }

    let selection = match tri {
        Tri::Nom => tete(lignes, k, sens, |row| store.nom(row)),
        Tri::Annee => tete(lignes, k, sens, |row| CleAnnee::new(store.annee(row))),
        Tri::Naturel => unreachable!("traité plus haut"),
    };
    selection.into_iter().skip(offset).collect()
}

fn parcourir(
    permutation: &crate::store::Permutation,
    lignes: &RoaringBitmap,
    tout: bool,
    sens: Sens,
    offset: usize,
    limite: usize,
) -> Vec<u32> {
    let (renseignees, absentes) = (permutation.renseignees(), permutation.absentes());
    let ordonnees: Box<dyn Iterator<Item = &u32>> = match sens {
        Sens::Croissant => Box::new(renseignees.iter().chain(absentes.iter())),
        Sens::Decroissant => Box::new(renseignees.iter().rev().chain(absentes.iter().rev())),
    };
    ordonnees
        .filter(|&&row| tout || lignes.contains(row))
        .skip(offset)
        .take(limite)
        .copied()
        .collect()
}

#[derive(PartialEq, Eq)]
struct CleAnnee {
    absente: bool,
    valeur: i32,
}

impl CleAnnee {
    fn new(a: Option<i32>) -> Self {
        Self {
            absente: a.is_none(),
            valeur: a.unwrap_or(i32::MIN),
        }
    }
}

impl Ord for CleAnnee {
    fn cmp(&self, other: &Self) -> Ordering {
        self.absente
            .cmp(&other.absente)
            .then_with(|| self.valeur.cmp(&other.valeur))
    }
}
impl PartialOrd for CleAnnee {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

trait CleTri: Ord {
    fn comparer(&self, autre: &Self, sens: Sens) -> Ordering;
}

impl CleTri for &str {
    fn comparer(&self, autre: &Self, sens: Sens) -> Ordering {
        match sens {
            Sens::Croissant => self.cmp(autre),
            Sens::Decroissant => autre.cmp(self),
        }
    }
}

impl CleTri for CleAnnee {
    fn comparer(&self, autre: &Self, sens: Sens) -> Ordering {
        self.absente.cmp(&autre.absente).then_with(|| match sens {
            Sens::Croissant => self.valeur.cmp(&autre.valeur),
            Sens::Decroissant => autre.valeur.cmp(&self.valeur),
        })
    }
}

fn tete<K: CleTri>(
    lignes: &RoaringBitmap,
    k: usize,
    sens: Sens,
    cle: impl Fn(u32) -> K,
) -> Vec<u32> {
    if k == 0 {
        return Vec::new();
    }
    let mut tas: BinaryHeap<Element<K>> = BinaryHeap::with_capacity(k.min(1024) + 1);
    for row in lignes {
        let element = Element {
            cle: cle(row),
            row,
            sens,
        };
        if tas.len() < k {
            tas.push(element);
        } else if let Some(sommet) = tas.peek()
            && element < *sommet
        {
            tas.pop();
            tas.push(element);
        }
    }
    let mut page: Vec<Element<K>> = tas.into_vec();
    page.sort_unstable();
    page.into_iter().map(|e| e.row).collect()
}

struct Element<K> {
    cle: K,
    row: u32,
    sens: Sens,
}

impl<K: CleTri> PartialEq for Element<K> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl<K: CleTri> Eq for Element<K> {}
impl<K: CleTri> PartialOrd for Element<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<K: CleTri> Ord for Element<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cle
            .comparer(&other.cle, self.sens)
            .then_with(|| match self.sens {
                Sens::Croissant => self.row.cmp(&other.row),
                Sens::Decroissant => other.row.cmp(&self.row),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::store_test;

    #[test]
    fn aucun_filtre_emprunte_le_bitmap_global() {
        let s = store_test();
        let r = resoudre(&s, &Filtres::default()).unwrap();
        assert_eq!(r.len(), 7);
        assert!(matches!(r, Cow::Borrowed(_)));
    }

    #[test]
    fn filtre_multi_valeurs_est_un_ou() {
        let s = store_test();
        let f = Filtres {
            races: vec!["Trotteur Francais".into(), "Pur Sang".into()],
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &f).unwrap().len(), 7);
    }

    #[test]
    fn filtres_de_dimensions_differentes_sont_un_et() {
        let s = store_test();
        let f = Filtres {
            races: vec!["Trotteur Francais".into()],
            sexes: vec!["Femelle".into()],
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &f).unwrap().len(), 2);
    }

    #[test]
    fn valeur_inconnue_rejetee_avec_indice() {
        let s = store_test();
        let f = Filtres {
            races: vec!["Licorne".into()],
            ..Default::default()
        };
        let err = resoudre(&s, &f).unwrap_err();
        assert_eq!(err.parametre, "race");
        assert!(err.indice.contains("/v1/referentiels/races"));
    }

    #[test]
    fn intervalle_de_millesimes() {
        let s = store_test();
        let f = Filtres {
            annee_min: Some(2005),
            annee_max: Some(2015),
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &f).unwrap().len(), 3);
    }

    #[test]
    fn filtre_sur_les_performances() {
        let s = store_test();
        let avec = Filtres {
            avec_performances: Some(true),
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &avec).unwrap().len(), 1);
        let sans = Filtres {
            avec_performances: Some(false),
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &sans).unwrap().len(), 6);
    }

    #[test]
    fn filtre_par_discipline() {
        let s = store_test();
        let f = Filtres {
            disciplines: vec!["TROT COURSE".into()],
            ..Default::default()
        };
        assert_eq!(resoudre(&s, &f).unwrap().len(), 1);
    }

    #[test]
    fn analyse_du_parametre_tri() {
        assert_eq!(Tri::parse("nom"), Some((Tri::Nom, Sens::Croissant)));
        assert_eq!(Tri::parse("-annee"), Some((Tri::Annee, Sens::Decroissant)));
        assert_eq!(Tri::parse("robe"), None);
    }

    fn reference(store: &Store, lignes: &RoaringBitmap, tri: Tri, sens: Sens) -> Vec<u32> {
        let absente = |row: u32| match tri {
            Tri::Annee => store.annee(row).is_none(),
            _ => false,
        };
        let selon_sens = |ord: Ordering| match sens {
            Sens::Croissant => ord,
            Sens::Decroissant => ord.reverse(),
        };
        let mut rows: Vec<u32> = lignes.iter().collect();
        rows.sort_by(|&a, &b| {
            absente(a)
                .cmp(&absente(b))
                .then(selon_sens(match tri {
                    Tri::Naturel => Ordering::Equal,
                    Tri::Nom => store.nom(a).cmp(store.nom(b)),
                    Tri::Annee => store.annee(a).cmp(&store.annee(b)),
                }))
                .then(selon_sens(a.cmp(&b)))
        });
        rows
    }

    #[test]
    fn les_deux_strategies_de_tri_donnent_le_meme_resultat() {
        let s = store_test();
        let selections = [
            ("tout", Filtres::default()),
            (
                "large",
                Filtres {
                    races: vec!["Trotteur Francais".into()],
                    ..Default::default()
                },
            ),
            (
                "etroite",
                Filtres {
                    avec_performances: Some(true),
                    ..Default::default()
                },
            ),
        ];
        for (nom_selection, filtres) in selections {
            let lignes = resoudre(&s, &filtres).unwrap();
            for tri in [Tri::Nom, Tri::Annee] {
                for sens in [Sens::Croissant, Sens::Decroissant] {
                    let attendu = reference(&s, &lignes, tri, sens);
                    for limite in [1, 2, 5, 50] {
                        for offset in [0, 1, 3] {
                            let obtenu = paginer(&s, &lignes, tri, sens, offset, limite);
                            let attendu_page: Vec<u32> =
                                attendu.iter().skip(offset).take(limite).copied().collect();
                            assert_eq!(
                                obtenu, attendu_page,
                                "sélection={nom_selection} tri={tri:?} sens={sens:?} \
                                 offset={offset} limite={limite}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn les_millesimes_absents_sont_toujours_en_dernier() {
        let s = store_test();
        let tous = resoudre(&s, &Filtres::default()).unwrap();
        for sens in [Sens::Croissant, Sens::Decroissant] {
            let page = paginer(&s, &tous, Tri::Annee, sens, 0, 10);
            let dernier = *page.last().unwrap();
            assert_eq!(
                s.annee(dernier),
                None,
                "la fiche sans millésime doit finir dernière (sens {sens:?})"
            );
        }
    }

    #[test]
    fn page_vide_au_dela_du_resultat() {
        let s = store_test();
        let tous = resoudre(&s, &Filtres::default()).unwrap();
        assert!(paginer(&s, &tous, Tri::Nom, Sens::Croissant, 100, 10).is_empty());
        assert!(paginer(&s, &tous, Tri::Naturel, Sens::Croissant, 100, 10).is_empty());
    }
}
