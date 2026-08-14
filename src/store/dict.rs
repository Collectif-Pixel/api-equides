use crate::text;
use std::collections::HashMap;

pub struct Dict {
    valeurs: Vec<String>,
    replies: HashMap<String, u32>,
}

impl Dict {
    pub(super) fn new(valeurs: Vec<String>) -> Self {
        let mut replies = HashMap::with_capacity(valeurs.len());
        for (code, libelle) in valeurs.iter().enumerate() {
            replies.entry(text::fold(libelle)).or_insert(code as u32);
        }
        Self { valeurs, replies }
    }

    #[inline]
    pub fn libelle(&self, code: u32) -> &str {
        self.valeurs.get(code as usize).map_or("", |s| s.as_str())
    }

    pub fn code(&self, libelle: &str) -> Option<u32> {
        self.replies.get(&text::fold(libelle)).copied()
    }

    pub fn len(&self) -> usize {
        self.valeurs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.valeurs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.valeurs
            .iter()
            .enumerate()
            .map(|(i, s)| (i as u32, s.as_str()))
    }
}
