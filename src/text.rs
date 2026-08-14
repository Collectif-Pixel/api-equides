use roaring::RoaringBitmap;
use std::collections::HashMap;

pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let up = c.to_ascii_uppercase();
        let folded = match up {
            'A'..='Z' | '0'..='9' => up,
            'À'..='Å' | 'Ā' | 'Ă' | 'Ą' | 'Æ' => 'A',
            'Ç' | 'Ć' | 'Č' => 'C',
            'È'..='Ë' | 'Ē' | 'Ė' | 'Ę' | 'Ě' => 'E',
            'Ì'..='Ï' | 'Ī' | 'Į' => 'I',
            'Ñ' | 'Ń' | 'Ň' => 'N',
            'Ò'..='Ö' | 'Ø' | 'Ō' | 'Ő' => 'O',
            'Ù'..='Ü' | 'Ū' | 'Ů' | 'Ű' => 'U',
            'Ý' | 'Ÿ' => 'Y',
            'Ž' | 'Ź' | 'Ż' => 'Z',
            'Ð' | 'Ď' => 'D',
            'Þ' => 'T',
            'Š' | 'Ś' | 'ß' => 'S',
            _ => ' ',
        };
        out.push(folded);
    }
    out
}

pub fn tokenize(folded: &str) -> impl Iterator<Item = &str> {
    folded.split(' ').filter(|t| !t.is_empty())
}

#[derive(Default)]
pub struct NameIndex {
    tokens: Vec<Box<str>>,
    postings: Vec<RoaringBitmap>,
}

impl NameIndex {
    pub fn build<'a>(noms: impl Iterator<Item = (u32, &'a str)>) -> Self {
        let mut map: HashMap<Box<str>, Vec<u32>> = HashMap::new();
        for (row, nom) in noms {
            let folded = fold(nom);
            let mut seen: Vec<&str> = Vec::new();
            for tok in tokenize(&folded) {
                if seen.contains(&tok) {
                    continue;
                }
                seen.push(tok);
                map.entry(tok.into()).or_default().push(row);
            }
        }

        let mut tokens: Vec<Box<str>> = map.keys().cloned().collect();
        tokens.sort_unstable();
        let postings = tokens
            .iter()
            .map(|t| {
                let mut rows = map.remove(t).unwrap_or_default();
                rows.sort_unstable();
                rows.dedup();
                RoaringBitmap::from_sorted_iter(rows).expect("lignes triées")
            })
            .collect();

        Self { tokens, postings }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    fn rows_with_prefix(&self, prefix: &str) -> RoaringBitmap {
        let start = self.tokens.partition_point(|t| t.as_ref() < prefix);
        let mut out = RoaringBitmap::new();
        for (tok, posting) in self.tokens[start..].iter().zip(&self.postings[start..]) {
            if !tok.starts_with(prefix) {
                break;
            }
            out |= posting;
        }
        out
    }

    pub fn suggerer(&self, query: &str, limite: usize) -> Vec<(u32, f32)> {
        let replie = fold(query);
        let jetons: Vec<&str> = tokenize(&replie).collect();
        let Some((prefixe, deja_saisis)) = jetons.split_last() else {
            return Vec::new();
        };

        let mut retenues: Option<RoaringBitmap> = None;
        for jeton in deja_saisis {
            let lignes = self.rows_with_prefix(jeton);
            let restreint = match retenues {
                None => lignes,
                Some(acc) => acc & lignes,
            };
            if restreint.is_empty() {
                return Vec::new();
            }
            retenues = Some(restreint);
        }

        let debut = self.tokens.partition_point(|t| t.as_ref() < *prefixe);
        let mut candidats: Vec<(usize, &str)> = self.tokens[debut..]
            .iter()
            .enumerate()
            .take_while(|(_, t)| t.starts_with(prefixe))
            .map(|(i, t)| (debut + i, t.as_ref()))
            .collect();

        candidats.sort_unstable_by_key(|(_, t)| (t.len(), *t));

        let mut vus = RoaringBitmap::new();
        let mut sorties: Vec<(u32, f32)> = Vec::with_capacity(limite);
        for (rang, (i, jeton)) in candidats.iter().enumerate() {
            if sorties.len() >= limite {
                break;
            }
            let score = if jeton == prefixe {
                1.0
            } else {
                let ecart = (jeton.len() - prefixe.len()) as f32;
                (1.0 / (1.0 + ecart)) * (1.0 / (1.0 + rang as f32 * 0.05))
            };
            for row in &self.postings[*i] {
                if sorties.len() >= limite {
                    break;
                }
                if retenues.as_ref().is_some_and(|r| !r.contains(row)) {
                    continue;
                }
                if vus.insert(row) {
                    sorties.push((row, score));
                }
            }
        }
        sorties
    }

    pub fn search(&self, query: &str) -> Option<RoaringBitmap> {
        let folded = fold(query);
        let mut result: Option<RoaringBitmap> = None;
        for tok in tokenize(&folded) {
            let rows = self.rows_with_prefix(tok);
            result = Some(match result {
                None => rows,
                Some(acc) => acc & rows,
            });
            if result
                .as_ref()
                .is_some_and(roaring::RoaringBitmap::is_empty)
            {
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repli_ascii() {
        assert_eq!(fold("ELDORÀDO"), "ELDORADO");
        assert_eq!(fold("ÈÉÎÙ"), "EEIU");
        assert_eq!(fold("grand brigadier"), "GRAND BRIGADIER");
        assert_eq!(fold("JEAN-LUC"), "JEAN LUC");
        assert_eq!(fold("KON. WARMB. NED"), "KON  WARMB  NED");
    }

    #[test]
    fn jetons() {
        let f = fold("KON. WARMB. NED");
        assert_eq!(
            tokenize(&f).collect::<Vec<_>>(),
            vec!["KON", "WARMB", "NED"]
        );
    }

    fn index_test() -> NameIndex {
        NameIndex::build(
            [
                (0u32, "GRAND BRIGADIER"),
                (1, "BRIGAND"),
                (2, "FELIX"),
                (3, "GRAND FELIX"),
                (4, "ELDORÀDO"),
            ]
            .into_iter(),
        )
    }

    #[test]
    fn recherche_par_prefixe() {
        let idx = index_test();
        let r = idx.search("BRIG").unwrap();
        assert_eq!(r.iter().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn recherche_multi_jetons_est_un_et() {
        let idx = index_test();
        let r = idx.search("GRAND FELIX").unwrap();
        assert_eq!(r.iter().collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn recherche_insensible_aux_accents_et_a_la_casse() {
        let idx = index_test();
        assert_eq!(
            idx.search("eldorado").unwrap().iter().collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(
            idx.search("ELDORÀDO").unwrap().iter().collect::<Vec<_>>(),
            vec![4]
        );
    }

    fn noms(idx: &NameIndex, requete: &str) -> Vec<u32> {
        let mut lignes: Vec<u32> = idx
            .suggerer(requete, 10)
            .into_iter()
            .map(|(r, _)| r)
            .collect();
        lignes.sort_unstable();
        lignes
    }

    #[test]
    fn suggestion_sur_un_seul_jeton() {
        let idx = index_test();
        assert_eq!(noms(&idx, "GRAND"), vec![0, 3]);
        assert_eq!(noms(&idx, "BRIG"), vec![0, 1]);
    }

    #[test]
    fn les_mots_deja_saisis_restreignent_la_suggestion() {
        let idx = index_test();
        assert_eq!(noms(&idx, "GRAND F"), vec![3]);
        assert_eq!(noms(&idx, "GRAND BRIG"), vec![0]);
        assert_eq!(noms(&idx, "FELIX GRAND"), vec![3]);
    }

    #[test]
    fn un_mot_saisi_sans_correspondance_ne_suggere_rien() {
        let idx = index_test();
        assert!(noms(&idx, "GRAND ZZZ").is_empty());
        assert!(noms(&idx, "ZZZ GRAND").is_empty());
    }

    #[test]
    fn le_dernier_mot_reste_un_prefixe() {
        let idx = index_test();
        assert_eq!(noms(&idx, "GRAND FEL"), vec![3]);
        assert_eq!(noms(&idx, "GRAND FELIX"), vec![3]);
    }

    #[test]
    fn recherche_sans_resultat_et_requete_vide() {
        let idx = index_test();
        assert!(idx.search("ZZZZ").unwrap().is_empty());
        assert!(idx.search("   ").is_none());
    }
}
