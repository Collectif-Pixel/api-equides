use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const BORNES: [f64; 9] = [0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0];

#[derive(Default)]
pub struct Metrics {
    total: AtomicU64,
    par_classe: [AtomicU64; 5],
    compartiments: [AtomicU64; BORNES.len()],
    somme_secondes_micro: AtomicU64,
    demarre: Option<Instant>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            demarre: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn observer(&self, statut: u16, duree_secondes: f64) {
        self.total.fetch_add(1, Ordering::Relaxed);
        let classe = ((statut / 100).clamp(1, 5) - 1) as usize;
        self.par_classe[classe].fetch_add(1, Ordering::Relaxed);
        self.somme_secondes_micro
            .fetch_add((duree_secondes * 1e6) as u64, Ordering::Relaxed);
        for (i, borne) in BORNES.iter().enumerate() {
            if duree_secondes <= *borne {
                self.compartiments[i].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn rendre(&self, lignes_jeu_de_donnees: u32) -> String {
        let total = self.total.load(Ordering::Relaxed);
        let mut s = String::with_capacity(2048);

        s.push_str("# HELP equides_requetes_total Requêtes HTTP servies.\n");
        s.push_str("# TYPE equides_requetes_total counter\n");
        for (i, nom) in ["1xx", "2xx", "3xx", "4xx", "5xx"].iter().enumerate() {
            let v = self.par_classe[i].load(Ordering::Relaxed);
            let _ = writeln!(s, "equides_requetes_total{{classe=\"{nom}\"}} {v}");
        }

        s.push_str("# HELP equides_duree_requete_secondes Latence de traitement.\n");
        s.push_str("# TYPE equides_duree_requete_secondes histogram\n");
        let mut cumul = 0u64;
        for (i, borne) in BORNES.iter().enumerate() {
            cumul = cumul.max(self.compartiments[i].load(Ordering::Relaxed));
            let _ = writeln!(
                s,
                "equides_duree_requete_secondes_bucket{{le=\"{borne}\"}} {cumul}"
            );
        }
        let _ = writeln!(
            s,
            "equides_duree_requete_secondes_bucket{{le=\"+Inf\"}} {total}"
        );
        let somme = self.somme_secondes_micro.load(Ordering::Relaxed) as f64 / 1e6;
        let _ = writeln!(s, "equides_duree_requete_secondes_sum {somme}");
        let _ = writeln!(s, "equides_duree_requete_secondes_count {total}");

        s.push_str("# HELP equides_lignes Nombre de lignes du jeu de données chargé.\n");
        s.push_str("# TYPE equides_lignes gauge\n");
        let _ = writeln!(s, "equides_lignes {lignes_jeu_de_donnees}");

        if let Some(t) = self.demarre {
            s.push_str("# HELP equides_duree_fonctionnement_secondes Temps depuis le démarrage.\n");
            s.push_str("# TYPE equides_duree_fonctionnement_secondes gauge\n");
            let _ = writeln!(
                s,
                "equides_duree_fonctionnement_secondes {}",
                t.elapsed().as_secs()
            );
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_compteurs_se_classent_par_statut() {
        let m = Metrics::new();
        m.observer(200, 0.0001);
        m.observer(200, 0.02);
        m.observer(404, 0.0001);
        m.observer(500, 2.0);
        let sortie = m.rendre(42);
        assert!(sortie.contains("equides_requetes_total{classe=\"2xx\"} 2"));
        assert!(sortie.contains("equides_requetes_total{classe=\"4xx\"} 1"));
        assert!(sortie.contains("equides_requetes_total{classe=\"5xx\"} 1"));
        assert!(sortie.contains("equides_duree_requete_secondes_count 4"));
        assert!(sortie.contains("equides_lignes 42"));
    }

    #[test]
    fn les_compartiments_sont_cumulatifs_et_croissants() {
        let m = Metrics::new();
        for d in [0.0001, 0.02, 2.0] {
            m.observer(200, d);
        }
        let sortie = m.rendre(0);
        let valeurs: Vec<u64> = sortie
            .lines()
            .filter(|l| l.starts_with("equides_duree_requete_secondes_bucket"))
            .filter_map(|l| l.rsplit(' ').next()?.parse().ok())
            .collect();
        assert!(
            valeurs.windows(2).all(|w| w[0] <= w[1]),
            "un histogramme Prometheus doit être cumulatif : {valeurs:?}"
        );
        assert_eq!(valeurs.last().copied(), Some(3));
    }
}
