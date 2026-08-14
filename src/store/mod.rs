pub mod dict;
pub mod index;

pub use dict::Dict;
use index::{Bitmaps, construire_bitmaps, construire_descendance, permutation_par_annee};
pub use index::{Descendance, Lien, MERE, PERE, PERE_DE_MERE, Permutation};

use crate::ids::{self, ID_BYTES};
use crate::snapshot::{ANNEE_ABSENTE, Arene, DatasetMeta, SANS_MODALITE, SANS_PARENT, Snapshot};
use crate::text::NameIndex;
use roaring::RoaringBitmap;
use std::collections::{BTreeMap, HashMap};

pub struct Store {
    pub meta: DatasetMeta,
    pub n: u32,

    pub races: Dict,
    pub robes: Dict,
    pub sexes: Dict,
    pub races_lien: Dict,
    pub pays_lien: Dict,
    pub disciplines: Dict,
    pub codes_indice: Dict,
    pub appreciations: Dict,

    race: Vec<u16>,
    robe: Vec<u16>,
    sexe: Vec<u8>,
    annee: Vec<i16>,

    pere: Vec<u32>,
    mere: Vec<u32>,
    pere_de_mere: Vec<u32>,
    lien_race: Vec<u16>,
    lien_pays: Vec<u16>,
    lien_alias: HashMap<(u32, u8), String>,
    liens_rompus: HashMap<(u32, u8), String>,

    ids: Vec<u8>,
    noms: Arene,
    slugs: Arene,
    filiations: Arene,

    perf_bornes: Vec<u32>,
    perf_discipline: Vec<u16>,
    perf_textes: Arene,
    ind_bornes: Vec<u32>,
    ind_code: Vec<u16>,
    ind_valeurs: Arene,
    ind_coefficient: Vec<f32>,
    ind_annee: Vec<i16>,
    ind_appreciation: Vec<u16>,

    id_order: Vec<u32>,

    idx_race: Vec<RoaringBitmap>,
    idx_robe: Vec<RoaringBitmap>,
    idx_sexe: Vec<RoaringBitmap>,
    idx_discipline: Vec<RoaringBitmap>,
    idx_code_indice: Vec<RoaringBitmap>,
    idx_annee: BTreeMap<i16, RoaringBitmap>,
    bm_tous: RoaringBitmap,
    bm_avec_performances: RoaringBitmap,

    tri_nom: Permutation,
    tri_annee: Permutation,

    pub descendance: Descendance,
    pub noms_index: NameIndex,
}

impl Store {
    pub fn from_snapshot(mut snap: Snapshot) -> Self {
        let n = snap.meta.lignes;
        let races = Dict::new(std::mem::take(&mut snap.dict_races));
        let robes = Dict::new(std::mem::take(&mut snap.dict_robes));
        let sexes = Dict::new(std::mem::take(&mut snap.dict_sexes));
        let races_lien = Dict::new(std::mem::take(&mut snap.dict_races_lien));
        let pays_lien = Dict::new(std::mem::take(&mut snap.dict_pays_lien));
        let disciplines = Dict::new(std::mem::take(&mut snap.dict_disciplines));
        let codes_indice = Dict::new(std::mem::take(&mut snap.dict_codes_indice));
        let appreciations = Dict::new(std::mem::take(&mut snap.dict_appreciations));

        let bm_tous = if n == 0 {
            RoaringBitmap::new()
        } else {
            RoaringBitmap::from_sorted_iter(0..n).expect("plage triée")
        };

        let (noms_index, tri_nom, tri_annee, id_order, descendance, bitmaps) =
            std::thread::scope(|portee| {
                let h_noms = portee.spawn(|| {
                    NameIndex::build((0..n).map(|row| (row, snap.noms.get(row as usize))))
                });
                let h_tri_nom = portee.spawn(|| {
                    let mut ordre: Vec<u32> = (0..n).collect();
                    ordre.sort_unstable_by(|&a, &b| {
                        snap.noms
                            .get(a as usize)
                            .cmp(snap.noms.get(b as usize))
                            .then(a.cmp(&b))
                    });
                    let non_nuls = ordre.len();
                    Permutation { ordre, non_nuls }
                });
                let h_tri_annee = portee.spawn(|| permutation_par_annee(&snap.annee));
                let h_ids = portee.spawn(|| {
                    let mut ordre: Vec<u32> = (0..n).collect();
                    ordre.sort_unstable_by(|&a, &b| {
                        tranche_id(&snap.ids, a).cmp(tranche_id(&snap.ids, b))
                    });
                    ordre
                });
                let h_desc = portee.spawn(|| construire_descendance(&snap));
                let bitmaps = construire_bitmaps(
                    &snap,
                    races.len(),
                    robes.len(),
                    sexes.len(),
                    disciplines.len(),
                    codes_indice.len(),
                );
                (
                    h_noms.join().expect("construction de l'index des noms"),
                    h_tri_nom.join().expect("tri par nom"),
                    h_tri_annee.join().expect("tri par millésime"),
                    h_ids.join().expect("tri par identifiant"),
                    h_desc.join().expect("index des descendants"),
                    bitmaps,
                )
            });

        let Bitmaps {
            idx_race,
            idx_robe,
            idx_sexe,
            idx_discipline,
            idx_code_indice,
            idx_annee,
            bm_avec_performances,
        } = bitmaps;

        Self {
            meta: snap.meta,
            n,
            races,
            robes,
            sexes,
            races_lien,
            pays_lien,
            disciplines,
            codes_indice,
            appreciations,
            race: snap.race,
            robe: snap.robe,
            sexe: snap.sexe,
            annee: snap.annee,
            pere: snap.pere,
            mere: snap.mere,
            pere_de_mere: snap.pere_de_mere,
            lien_race: snap.lien_race,
            lien_pays: snap.lien_pays,
            lien_alias: std::mem::take(&mut snap.lien_alias)
                .into_iter()
                .map(|(r, g, v)| ((r, g), v))
                .collect(),
            liens_rompus: std::mem::take(&mut snap.liens_rompus)
                .into_iter()
                .map(|(r, g, v)| ((r, g), v))
                .collect(),
            ids: snap.ids,
            noms: snap.noms,
            slugs: snap.slugs,
            filiations: snap.filiations,
            perf_bornes: snap.perf_bornes,
            perf_discipline: snap.perf_discipline,
            perf_textes: snap.perf_textes,
            ind_bornes: snap.ind_bornes,
            ind_code: snap.ind_code,
            ind_valeurs: snap.ind_valeurs,
            ind_coefficient: snap.ind_coefficient,
            ind_annee: snap.ind_annee,
            ind_appreciation: snap.ind_appreciation,
            id_order,
            idx_race,
            idx_robe,
            idx_sexe,
            idx_discipline,
            idx_code_indice,
            idx_annee,
            bm_tous,
            bm_avec_performances,
            tri_nom,
            tri_annee,
            descendance,
            noms_index,
        }
    }

    #[inline]
    pub fn nom(&self, row: u32) -> &str {
        self.noms.get(row as usize)
    }

    #[inline]
    pub fn slug(&self, row: u32) -> &str {
        self.slugs.get(row as usize)
    }

    #[inline]
    pub fn filiation_texte(&self, row: u32) -> &str {
        self.filiations.get(row as usize)
    }

    #[inline]
    pub fn race_libelle(&self, row: u32) -> &str {
        libelle_ou_vide(&self.races, self.race[row as usize])
    }

    #[inline]
    pub fn robe_libelle(&self, row: u32) -> &str {
        libelle_ou_vide(&self.robes, self.robe[row as usize])
    }

    #[inline]
    pub fn sexe_libelle(&self, row: u32) -> &str {
        match self.sexe[row as usize] {
            u8::MAX => "",
            code => self.sexes.libelle(code as u32),
        }
    }

    #[inline]
    pub fn annee(&self, row: u32) -> Option<i32> {
        match self.annee[row as usize] {
            ANNEE_ABSENTE => None,
            a => Some(a as i32),
        }
    }

    #[inline]
    pub fn id(&self, row: u32) -> String {
        let mut buf = [0u8; ID_BYTES];
        buf.copy_from_slice(tranche_id(&self.ids, row));
        ids::encode(&buf)
    }

    pub fn url(&self, row: u32) -> String {
        self.meta
            .url_modele
            .replace("{slug}", self.slug(row))
            .replace("{id}", &self.id(row))
    }

    pub fn row_by_id(&self, id: &str) -> Option<u32> {
        let cherche = ids::decode(id)?;
        let pos = self
            .id_order
            .binary_search_by(|&row| tranche_id(&self.ids, row).cmp(&cherche[..]))
            .ok()?;
        Some(self.id_order[pos])
    }

    pub fn lien(&self, row: u32, genre: usize) -> Option<Lien<'_>> {
        let i = row as usize * 3 + genre;
        let cible = sans_sentinelle(match genre {
            PERE => self.pere[row as usize],
            MERE => self.mere[row as usize],
            _ => self.pere_de_mere[row as usize],
        });
        let cle = (row, genre as u8);
        let rompu = self.liens_rompus.get(&cle).map(String::as_str);
        let race_lien = libelle_ou_vide(&self.races_lien, self.lien_race[i]);
        let pays = libelle_ou_vide(&self.pays_lien, self.lien_pays[i]);

        if cible.is_none() && rompu.is_none() && race_lien.is_empty() && pays.is_empty() {
            return None;
        }
        Some(Lien {
            cible,
            nom: match cible {
                Some(c) => self.nom(c),
                None => rompu.unwrap_or(""),
            },
            race: race_lien,
            pays: (!pays.is_empty()).then_some(pays),
            alias: self.lien_alias.get(&cle).map(String::as_str),
        })
    }

    #[inline]
    pub fn pere(&self, row: u32) -> Option<u32> {
        sans_sentinelle(self.pere[row as usize])
    }

    #[inline]
    pub fn mere(&self, row: u32) -> Option<u32> {
        sans_sentinelle(self.mere[row as usize])
    }

    #[inline]
    pub fn pere_de_mere(&self, row: u32) -> Option<u32> {
        sans_sentinelle(self.pere_de_mere[row as usize])
    }

    #[inline]
    pub fn enfants(&self, row: u32) -> &[u32] {
        self.descendance.enfants(row)
    }

    #[inline]
    pub fn performances(&self, row: u32) -> std::ops::Range<usize> {
        self.perf_bornes[row as usize] as usize..self.perf_bornes[row as usize + 1] as usize
    }

    #[inline]
    pub fn perf_discipline(&self, i: usize) -> &str {
        libelle_ou_vide(&self.disciplines, self.perf_discipline[i])
    }

    #[inline]
    pub fn perf_texte(&self, i: usize) -> &str {
        self.perf_textes.get(i)
    }

    #[inline]
    pub fn indices(&self, perf: usize) -> std::ops::Range<usize> {
        self.ind_bornes[perf] as usize..self.ind_bornes[perf + 1] as usize
    }

    #[inline]
    pub fn indice_code(&self, i: usize) -> &str {
        libelle_ou_vide(&self.codes_indice, self.ind_code[i])
    }

    #[inline]
    pub fn indice_valeur(&self, i: usize) -> &str {
        self.ind_valeurs.get(i)
    }

    #[inline]
    pub fn indice_coefficient(&self, i: usize) -> Option<f32> {
        let c = self.ind_coefficient[i];
        c.is_finite().then_some(c)
    }

    #[inline]
    pub fn indice_annee(&self, i: usize) -> Option<i32> {
        match self.ind_annee[i] {
            ANNEE_ABSENTE => None,
            a => Some(a as i32),
        }
    }

    #[inline]
    pub fn indice_appreciation(&self, i: usize) -> &str {
        libelle_ou_vide(&self.appreciations, self.ind_appreciation[i])
    }

    pub fn toutes_les_lignes(&self) -> &RoaringBitmap {
        &self.bm_tous
    }

    pub fn bitmap_race(&self, code: u32) -> Option<&RoaringBitmap> {
        self.idx_race.get(code as usize)
    }

    pub fn bitmap_robe(&self, code: u32) -> Option<&RoaringBitmap> {
        self.idx_robe.get(code as usize)
    }

    pub fn bitmap_sexe(&self, code: u32) -> Option<&RoaringBitmap> {
        self.idx_sexe.get(code as usize)
    }

    pub fn bitmap_discipline(&self, code: u32) -> Option<&RoaringBitmap> {
        self.idx_discipline.get(code as usize)
    }

    pub fn bitmap_code_indice(&self, code: u32) -> Option<&RoaringBitmap> {
        self.idx_code_indice.get(code as usize)
    }

    pub fn bitmap_annee(&self, annee: i32) -> Option<&RoaringBitmap> {
        i16::try_from(annee)
            .ok()
            .and_then(|a| self.idx_annee.get(&a))
    }

    pub fn bitmap_avec_performances(&self) -> &RoaringBitmap {
        &self.bm_avec_performances
    }

    pub fn annees(&self) -> impl Iterator<Item = (i32, u64)> {
        self.idx_annee.iter().map(|(a, b)| (*a as i32, b.len()))
    }

    pub fn bitmaps_annees(&self) -> impl Iterator<Item = (i32, &RoaringBitmap)> {
        self.idx_annee.iter().map(|(a, b)| (*a as i32, b))
    }

    pub fn permutation_nom(&self) -> &Permutation {
        &self.tri_nom
    }

    pub fn permutation_annee(&self) -> &Permutation {
        &self.tri_annee
    }

    pub fn empreinte(&self) -> &str {
        &self.meta.empreinte_source
    }
}

#[inline]
fn sans_sentinelle(v: u32) -> Option<u32> {
    (v != SANS_PARENT).then_some(v)
}

#[inline]
fn libelle_ou_vide(dict: &Dict, code: u16) -> &str {
    match code {
        SANS_MODALITE => "",
        c => dict.libelle(c as u32),
    }
}

#[inline]
fn tranche_id(ids: &[u8], row: u32) -> &[u8] {
    let debut = row as usize * ID_BYTES;
    &ids[debut..debut + ID_BYTES]
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::snapshot::{Anomalies, COEFFICIENT_ABSENT, FORMAT_VERSION};

    type LigneTest = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        i32,
        &'static str,
        &'static str,
    );

    const LIGNES: &[LigneTest] = &[
        (
            "AAAAAAAAAAAAAAAAAAAAAA",
            "GRAND PERE",
            "Trotteur Francais",
            "Male",
            "Bai",
            1990,
            "",
            "",
        ),
        (
            "BBBBBBBBBBBBBBBBBBBBBA",
            "GRAND MERE",
            "Trotteur Francais",
            "Femelle",
            "Alezan",
            1992,
            "",
            "",
        ),
        (
            "CCCCCCCCCCCCCCCCCCCCCA",
            "PERE",
            "Trotteur Francais",
            "Male",
            "Bai",
            2005,
            "AAAAAAAAAAAAAAAAAAAAAA",
            "BBBBBBBBBBBBBBBBBBBBBA",
        ),
        (
            "DDDDDDDDDDDDDDDDDDDDDA",
            "MERE",
            "Pur Sang",
            "Femelle",
            "Gris",
            2006,
            "",
            "",
        ),
        (
            "EEEEEEEEEEEEEEEEEEEEEA",
            "ELDORÀDO",
            "Trotteur Francais",
            "Femelle",
            "Bai",
            2015,
            "CCCCCCCCCCCCCCCCCCCCCA",
            "DDDDDDDDDDDDDDDDDDDDDA",
        ),
        (
            "FFFFFFFFFFFFFFFFFFFFFA",
            "ORPHELIN",
            "Pur Sang",
            "Hongre",
            "",
            2020,
            "ZZZZZZZZZZZZZZZZZZZZZZ",
            "",
        ),
        (
            "GGGGGGGGGGGGGGGGGGGGGA",
            "SANS DATE",
            "Pur Sang",
            "Indeter",
            "Noir",
            0,
            "",
            "",
        ),
    ];

    pub fn store_test() -> Store {
        let mut dicts: [Vec<String>; 4] = Default::default();
        let intern = |d: &mut Vec<String>, v: &str| -> u16 {
            if let Some(i) = d.iter().position(|x| x == v) {
                i as u16
            } else {
                d.push(v.to_string());
                (d.len() - 1) as u16
            }
        };

        let n = LIGNES.len();
        let mut ids_plats = Vec::new();
        let mut noms = Arene::nouvelle();
        let mut slugs = Arene::nouvelle();
        let mut filiations = Arene::nouvelle();
        let (mut race, mut robe, mut sexe, mut annee) = (vec![], vec![], vec![], vec![]);
        let mut index: HashMap<&str, u32> = HashMap::new();

        for (i, l) in LIGNES.iter().enumerate() {
            index.insert(l.0, i as u32);
            ids_plats.extend_from_slice(&ids::decode(l.0).expect("identifiant de test valide"));
            noms.pousser(l.1).unwrap();
            slugs
                .pousser(&l.1.to_lowercase().replace(' ', "-"))
                .unwrap();
            filiations.pousser("").unwrap();
            race.push(intern(&mut dicts[0], l.2));
            sexe.push(intern(&mut dicts[2], l.3) as u8);
            robe.push(if l.4.is_empty() {
                SANS_MODALITE
            } else {
                intern(&mut dicts[1], l.4)
            });
            annee.push(if l.5 == 0 { ANNEE_ABSENTE } else { l.5 as i16 });
        }
        let lien = |cible: &str| -> u32 { index.get(cible).copied().unwrap_or(SANS_PARENT) };
        let pere: Vec<u32> = LIGNES.iter().map(|l| lien(l.6)).collect();
        let mere: Vec<u32> = LIGNES.iter().map(|l| lien(l.7)).collect();

        let [dict_races, dict_robes, dict_sexes, _] = dicts;

        let lien_race: Vec<u16> = (0..n * 3)
            .map(|i| {
                let declaree = match i % 3 {
                    0 => pere[i / 3] != SANS_PARENT,
                    1 => mere[i / 3] != SANS_PARENT,
                    _ => false,
                };
                if declaree { 0 } else { SANS_MODALITE }
            })
            .collect();

        let mut perf_bornes = vec![0u32];
        let mut perf_discipline = Vec::new();
        let mut perf_textes = Arene::nouvelle();
        let mut ind_bornes = vec![0u32];
        let mut ind_code = Vec::new();
        let mut ind_valeurs = Arene::nouvelle();
        for i in 0..n {
            if i == 4 {
                perf_discipline.push(0);
                perf_textes.pousser("BTR +59 (0.35)").unwrap();
                ind_code.push(0);
                ind_valeurs.pousser("+59").unwrap();
                ind_bornes.push(ind_code.len() as u32);
            }
            perf_bornes.push(perf_discipline.len() as u32);
        }
        let nb_ind = ind_code.len();

        Store::from_snapshot(Snapshot {
            format_version: FORMAT_VERSION,
            meta: DatasetMeta {
                producteur: "IFCE".into(),
                source: "test".into(),
                licence: "Non déterminée".into(),
                url_modele: "https://infochevaux.ifce.fr/fr/{slug}-{id}/infos-generales".into(),
                ingere_le: "2026-01-01".into(),
                empreinte_source: "test".into(),
                lignes: n as u32,
                anomalies: Anomalies {
                    parents_pendants: 1,
                    ..Default::default()
                },
            },
            dict_races,
            dict_robes,
            dict_sexes,
            dict_races_lien: vec!["TF".into(), "PS".into()],
            dict_pays_lien: vec!["IRLANDE".into()],
            dict_disciplines: vec!["TROT COURSE".into()],
            dict_codes_indice: vec!["BTR".into()],
            dict_appreciations: vec![],
            ids: ids_plats,
            noms,
            slugs,
            filiations,
            race,
            robe,
            sexe,
            annee,
            pere,
            mere,
            pere_de_mere: vec![SANS_PARENT; n],
            lien_race,
            lien_pays: (0..n * 3)
                .map(|i| if i == 4 * 3 + 1 { 0 } else { SANS_MODALITE })
                .collect(),
            lien_alias: vec![],
            liens_rompus: vec![(5, 0, "PERE INTROUVABLE".to_string())],
            perf_bornes,
            perf_discipline,
            perf_textes,
            ind_bornes,
            ind_code,
            ind_valeurs,
            ind_coefficient: vec![COEFFICIENT_ABSENT; nb_ind],
            ind_annee: vec![ANNEE_ABSENTE; nb_ind],
            ind_appreciation: vec![SANS_MODALITE; nb_ind],
        })
    }

    #[test]
    fn acces_colonnes() {
        let s = store_test();
        assert_eq!(s.n, 7);
        assert_eq!(s.nom(4), "ELDORÀDO");
        assert_eq!(s.race_libelle(4), "Trotteur Francais");
        assert_eq!(s.sexe_libelle(4), "Femelle");
        assert_eq!(s.annee(4), Some(2015));
        assert_eq!(s.robe_libelle(5), "");
        assert_eq!(s.annee(6), None);
    }

    #[test]
    fn identifiants_officiels_conserves() {
        let s = store_test();
        assert_eq!(s.id(4), "EEEEEEEEEEEEEEEEEEEEEA");
        for row in 0..s.n {
            assert_eq!(s.row_by_id(&s.id(row)), Some(row));
        }
        assert_eq!(s.row_by_id("ZZZZZZZZZZZZZZZZZZZZZA"), None);
        assert_eq!(s.row_by_id("pas-un-identifiant"), None);
    }

    #[test]
    fn url_reconstruite() {
        let s = store_test();
        let url = s.url(4);
        assert!(url.starts_with("https://infochevaux.ifce.fr/fr/"));
        assert!(url.ends_with("/infos-generales"));
        assert!(url.contains(s.slug(4)));
        assert!(url.contains(&s.id(4)));
    }

    #[test]
    fn filiation_resolue_en_indices() {
        let s = store_test();
        assert_eq!(s.pere(4).map(|r| s.nom(r)), Some("PERE"));
        assert_eq!(s.mere(4).map(|r| s.nom(r)), Some("MERE"));
        let pere = s.pere(4).unwrap();
        assert_eq!(s.pere(pere).map(|r| s.nom(r)), Some("GRAND PERE"));
        assert_eq!(s.mere(pere).map(|r| s.nom(r)), Some("GRAND MERE"));
        assert_eq!(s.pere(0), None);
    }

    #[test]
    fn parent_pendant_coupe_le_lien() {
        let s = store_test();
        let orphelin = s.row_by_id("FFFFFFFFFFFFFFFFFFFFFA").unwrap();
        assert_eq!(s.pere(orphelin), None);
    }

    #[test]
    fn descendance_inverse_la_filiation() {
        let s = store_test();
        let grand_pere = s.row_by_id("AAAAAAAAAAAAAAAAAAAAAA").unwrap();
        let enfants: Vec<&str> = s.enfants(grand_pere).iter().map(|&r| s.nom(r)).collect();
        assert_eq!(enfants, vec!["PERE"]);

        let pere = s.row_by_id("CCCCCCCCCCCCCCCCCCCCCA").unwrap();
        assert_eq!(
            s.enfants(pere)
                .iter()
                .map(|&r| s.nom(r))
                .collect::<Vec<_>>(),
            vec!["ELDORÀDO"]
        );
        assert!(
            s.enfants(s.row_by_id("EEEEEEEEEEEEEEEEEEEEEA").unwrap())
                .is_empty()
        );
    }

    #[test]
    fn performances() {
        let s = store_test();
        let eldorado = s.row_by_id("EEEEEEEEEEEEEEEEEEEEEA").unwrap();
        let perfs: Vec<usize> = s.performances(eldorado).collect();
        assert_eq!(perfs.len(), 1);
        assert_eq!(s.perf_discipline(perfs[0]), "TROT COURSE");
        assert_eq!(s.perf_texte(perfs[0]), "BTR +59 (0.35)");
        let inds: Vec<usize> = s.indices(perfs[0]).collect();
        assert_eq!(s.indice_code(inds[0]), "BTR");
        assert_eq!(s.indice_valeur(inds[0]), "+59");
        assert_eq!(s.indice_coefficient(inds[0]), None);
        assert!(s.performances(0).is_empty());
    }

    #[test]
    fn index_bitmap_coherents() {
        let s = store_test();
        let tf = s.races.code("trotteur francais").unwrap();
        assert_eq!(s.bitmap_race(tf).unwrap().len(), 4);
        assert_eq!(s.bitmap_annee(2015).unwrap().len(), 1);
        assert_eq!(s.bitmap_annee(1789), None);
        assert_eq!(s.bitmap_avec_performances().len(), 1);
        assert_eq!(s.toutes_les_lignes().len(), 7);
        let trot = s.disciplines.code("TROT COURSE").unwrap();
        assert_eq!(s.bitmap_discipline(trot).unwrap().len(), 1);
    }

    #[test]
    fn dictionnaire_insensible_casse_et_accents() {
        let s = store_test();
        assert_eq!(s.races.code("PUR SANG"), s.races.code("Pur Sang"));
        assert_eq!(s.races.code("Race Inexistante"), None);
    }

    #[test]
    fn index_des_noms() {
        let s = store_test();
        let r = s.noms_index.search("eldorado").unwrap();
        assert_eq!(r.iter().collect::<Vec<_>>(), vec![4]);
        assert_eq!(s.noms_index.search("grand").unwrap().len(), 2);
    }
}
