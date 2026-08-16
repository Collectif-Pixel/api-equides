use crate::snapshot::{ANNEE_ABSENTE, SANS_MODALITE, SANS_PARENT, Snapshot};
use roaring::RoaringBitmap;
use std::collections::BTreeMap;

pub struct Permutation {
    pub(super) ordre: Vec<u32>,
    pub(super) non_nuls: usize,
}

impl Permutation {
    #[inline]
    pub fn renseignees(&self) -> &[u32] {
        &self.ordre[..self.non_nuls]
    }

    #[inline]
    pub fn absentes(&self) -> &[u32] {
        &self.ordre[self.non_nuls..]
    }
}

pub const PERE: usize = 0;
pub const MERE: usize = 1;
pub const PERE_DE_MERE: usize = 2;

pub struct Lien<'a> {
    pub cible: Option<u32>,
    pub nom: &'a str,
    pub race: &'a str,
    pub pays: Option<&'a str>,
    pub alias: Option<&'a str>,
}

pub struct Descendance {
    pub(super) bornes: Vec<u32>,
    pub(super) aretes: Vec<u32>,
}

impl Descendance {
    #[inline]
    pub fn enfants(&self, row: u32) -> &[u32] {
        let (d, f) = (
            self.bornes[row as usize] as usize,
            self.bornes[row as usize + 1] as usize,
        );
        &self.aretes[d..f]
    }
}

pub(super) struct Bitmaps {
    pub(super) idx_race: Vec<RoaringBitmap>,
    pub(super) idx_robe: Vec<RoaringBitmap>,
    pub(super) idx_sexe: Vec<RoaringBitmap>,
    pub(super) idx_statut_reproducteur: Vec<RoaringBitmap>,
    pub(super) idx_discipline: Vec<RoaringBitmap>,
    pub(super) idx_code_indice: Vec<RoaringBitmap>,
    pub(super) idx_annee: BTreeMap<i16, RoaringBitmap>,
    pub(super) bm_avec_performances: RoaringBitmap,
    pub(super) bm_avec_record: RoaringBitmap,
}

pub(super) struct Tailles {
    pub(super) races: usize,
    pub(super) robes: usize,
    pub(super) sexes: usize,
    pub(super) statuts_reproducteur: usize,
    pub(super) disciplines: usize,
    pub(super) codes_indice: usize,
}

pub(super) fn construire_bitmaps(snap: &Snapshot, tailles: &Tailles) -> Bitmaps {
    let mut b = Bitmaps {
        idx_race: vec![RoaringBitmap::new(); tailles.races],
        idx_robe: vec![RoaringBitmap::new(); tailles.robes],
        idx_sexe: vec![RoaringBitmap::new(); tailles.sexes],
        idx_statut_reproducteur: vec![RoaringBitmap::new(); tailles.statuts_reproducteur],
        idx_discipline: vec![RoaringBitmap::new(); tailles.disciplines],
        idx_code_indice: vec![RoaringBitmap::new(); tailles.codes_indice],
        idx_annee: BTreeMap::new(),
        bm_avec_performances: RoaringBitmap::new(),
        bm_avec_record: RoaringBitmap::new(),
    };
    const CROISSANT: &str = "insertion en ordre croissant";
    for row in 0..snap.meta.lignes {
        let i = row as usize;
        if snap.race[i] != SANS_MODALITE {
            b.idx_race[snap.race[i] as usize]
                .try_push(row)
                .expect(CROISSANT);
        }
        if snap.robe[i] != SANS_MODALITE {
            b.idx_robe[snap.robe[i] as usize]
                .try_push(row)
                .expect(CROISSANT);
        }
        if snap.sexe[i] != u8::MAX {
            b.idx_sexe[snap.sexe[i] as usize]
                .try_push(row)
                .expect(CROISSANT);
        }
        if !snap.records.get(i).is_empty() {
            b.bm_avec_record.try_push(row).expect(CROISSANT);
        }
        if snap.statut_reproducteur[i] != SANS_MODALITE {
            b.idx_statut_reproducteur[snap.statut_reproducteur[i] as usize]
                .try_push(row)
                .expect(CROISSANT);
        }
        if snap.annee[i] != ANNEE_ABSENTE {
            b.idx_annee
                .entry(snap.annee[i])
                .or_default()
                .try_push(row)
                .expect(CROISSANT);
        }
        let perfs = snap.perf_bornes[i] as usize..snap.perf_bornes[i + 1] as usize;
        if !perfs.is_empty() {
            b.bm_avec_performances.try_push(row).expect(CROISSANT);
            for p in perfs {
                if snap.perf_discipline[p] != SANS_MODALITE {
                    b.idx_discipline[snap.perf_discipline[p] as usize].insert(row);
                }
                for i in snap.ind_bornes[p] as usize..snap.ind_bornes[p + 1] as usize {
                    if snap.ind_code[i] != SANS_MODALITE {
                        b.idx_code_indice[snap.ind_code[i] as usize].insert(row);
                    }
                }
            }
        }
    }
    b
}

pub(super) fn construire_descendance(snap: &Snapshot) -> Descendance {
    let n = snap.meta.lignes as usize;
    let mut bornes = vec![0u32; n + 2];

    let parents_de = |i: usize| [snap.pere[i], snap.mere[i]];
    for i in 0..n {
        for p in parents_de(i) {
            if p != SANS_PARENT {
                bornes[p as usize + 1] += 1;
            }
        }
    }
    for i in 0..=n {
        bornes[i + 1] += bornes[i];
    }

    let total = bornes[n] as usize;
    let mut aretes = vec![0u32; total];
    let mut curseur = bornes.clone();
    for i in 0..n {
        for p in parents_de(i) {
            if p != SANS_PARENT {
                aretes[curseur[p as usize] as usize] = i as u32;
                curseur[p as usize] += 1;
            }
        }
    }
    bornes.truncate(n + 1);
    Descendance { bornes, aretes }
}

pub(super) fn permutation_par_annee(annees: &[i16]) -> Permutation {
    let mut renseignees: Vec<u32> = Vec::with_capacity(annees.len());
    let mut absentes: Vec<u32> = Vec::new();
    for (row, &a) in annees.iter().enumerate() {
        if a == ANNEE_ABSENTE {
            absentes.push(row as u32);
        } else {
            renseignees.push(row as u32);
        }
    }
    renseignees.sort_unstable_by_key(|&r| (annees[r as usize], r));
    let non_nuls = renseignees.len();
    renseignees.extend_from_slice(&absentes);
    Permutation {
        ordre: renseignees,
        non_nuls,
    }
}
