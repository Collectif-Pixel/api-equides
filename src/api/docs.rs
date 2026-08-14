use serde_json::Value;
use std::fmt::Write as _;

fn h(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn rendre(doc: &Value, base: &str) -> String {
    let info = &doc["info"];
    let titre = info["title"].as_str().unwrap_or("API");
    let version = info["version"].as_str().unwrap_or("");

    let mut p = String::with_capacity(64 * 1024);
    let _ = write!(
        p,
        r#"<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{titre} — documentation</title>
<style>{STYLE}</style></head><body>
<header><h1>{titre} <span class="v">v{version}</span></h1>"#,
        titre = h(titre),
        version = h(version),
    );

    if let Some(d) = info["description"].as_str() {
        let _ = write!(p, "<div class=\"intro\">{}</div>", markdown_leger(d));
    }
    let _ = write!(
        p,
        r#"<p class="liens"><a href="/openapi.json">openapi.json</a> · <a href="/v1/meta">métadonnées</a></p></header>"#
    );

    let etiquettes: Vec<(&str, &str)> = doc["tags"]
        .as_array()
        .map(|t| {
            t.iter()
                .map(|x| {
                    (
                        x["name"].as_str().unwrap_or(""),
                        x["description"].as_str().unwrap_or(""),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let chemins = doc["paths"].as_object();
    let operations = |etiquette: &str| -> Vec<(String, &Value)> {
        chemins
            .map(|c| {
                c.iter()
                    .filter_map(|(chemin, item)| {
                        let op = item.get("get")?;
                        let porte = op["tags"]
                            .as_array()
                            .is_some_and(|t| t.iter().any(|x| x.as_str() == Some(etiquette)));
                        porte.then(|| (chemin.clone(), op))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let erreurs = doc["x-erreurs"].as_array().cloned().unwrap_or_default();

    let _ = write!(p, "<nav><ul>");
    for (nom, _) in &etiquettes {
        let _ = write!(p, "<li><a href=\"#{0}\">{0}</a><ul>", h(nom));
        for (chemin, _) in operations(nom) {
            let _ = write!(
                p,
                "<li><a href=\"#{}\"><code>{}</code></a></li>",
                ancre(&chemin),
                h(&chemin)
            );
        }
        let _ = write!(p, "</ul></li>");
    }
    if !erreurs.is_empty() {
        let _ = write!(p, "<li><a href=\"#erreurs\">Erreurs</a><ul>");
        for e in &erreurs {
            let _ = write!(
                p,
                "<li><a href=\"#{}\">{}</a></li>",
                h(e["ancre"].as_str().unwrap_or("")),
                h(e["titre"].as_str().unwrap_or(""))
            );
        }
        let _ = write!(p, "</ul></li>");
    }
    let _ = write!(p, "</ul></nav><main>");

    for (nom, description) in &etiquettes {
        let ops = operations(nom);
        if ops.is_empty() {
            continue;
        }
        let _ = write!(p, "<section><h2 id=\"{}\">{}</h2>", h(nom), h(nom));
        if !description.is_empty() {
            let _ = write!(p, "<p class=\"desc\">{}</p>", h(description));
        }
        for (chemin, op) in ops {
            rendre_operation(&mut p, &chemin, op, base);
        }
        let _ = write!(p, "</section>");
    }

    rendre_erreurs(&mut p, &erreurs);

    let _ = write!(
        p,
        "</main><footer><p>API Équidés · licence MIT · par le Collectif Pixel</p></footer></body></html>"
    );
    p
}

fn ancre(chemin: &str) -> String {
    chemin
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn rendre_erreurs(p: &mut String, erreurs: &[Value]) {
    if erreurs.is_empty() {
        return;
    }
    let _ = write!(
        p,
        "<section><h2 id=\"erreurs\">Erreurs</h2>\
         <p class=\"desc\">Toutes les erreurs suivent la RFC 9457 (« Problem Details for HTTP \
         APIs ») et sont servies en <code>application/problem+json</code>. Leur champ \
         <code>type</code> désigne l'une des sections ci-dessous ; <code>detail</code> explique \
         le cas rencontré, et <code>parametre</code> et <code>indice</code> — extensions que la \
         RFC autorise — nomment le paramètre fautif et proposent la correction.</p>"
    );
    for e in erreurs {
        let statut = e["statut"].as_u64().unwrap_or(0);
        let classe = if (200..300).contains(&statut) {
            "ok"
        } else {
            "ko"
        };
        let _ = write!(
            p,
            "<article id=\"{ancre}\"><h3><span class=\"code {classe}\">{statut}</span> {titre}</h3>\
             <div class=\"desc\">{quand}</div>\
             <p class=\"desc\"><code>\"type\": \"{uri}\"</code></p></article>",
            ancre = h(e["ancre"].as_str().unwrap_or("")),
            titre = h(e["titre"].as_str().unwrap_or("")),
            quand = markdown_leger(e["quand"].as_str().unwrap_or("")),
            uri = h(e["uri"].as_str().unwrap_or("")),
        );
    }
    let _ = write!(p, "</section>");
}

fn rendre_operation(p: &mut String, chemin: &str, op: &Value, base: &str) {
    let resume = op["summary"].as_str().unwrap_or("");
    let _ = write!(
        p,
        "<article id=\"{}\"><h3><span class=\"m\">GET</span> <code>{}</code></h3>",
        ancre(chemin),
        h(chemin)
    );
    if !resume.is_empty() {
        let _ = write!(p, "<p class=\"resume\">{}</p>", h(resume));
    }
    if let Some(d) = op["description"].as_str() {
        let _ = write!(p, "<div class=\"desc\">{}</div>", markdown_leger(d));
    }

    let params = op["parameters"].as_array().cloned().unwrap_or_default();
    let dans = |ou: &str| -> Vec<&Value> { params.iter().filter(|x| x["in"] == ou).collect() };

    for (titre, liste) in [
        ("Paramètres de chemin", dans("path")),
        ("En-têtes", dans("header")),
        ("Paramètres", dans("query")),
    ] {
        if liste.is_empty() {
            continue;
        }
        let _ = write!(
            p,
            "<h4>{titre}</h4><table><thead><tr><th>nom</th><th>type</th><th>description</th></tr></thead><tbody>"
        );
        for x in liste {
            let requis = if x["required"] == true {
                " <span class=\"req\">requis</span>"
            } else {
                ""
            };
            let mut colonne_type = h(&type_lisible(&x["schema"]));
            for c in contraintes(&x["schema"]) {
                let _ = write!(colonne_type, "<span class=\"contr\">{}</span>", h(&c));
            }
            let _ = write!(
                p,
                "<tr><td><code>{}</code>{}</td><td class=\"t\">{}</td><td>{}</td></tr>",
                h(x["name"].as_str().unwrap_or("")),
                requis,
                colonne_type,
                markdown_leger(x["description"].as_str().unwrap_or(""))
            );
        }
        let _ = write!(p, "</tbody></table>");
    }

    if let Some(reponses) = op["responses"].as_object() {
        let _ = write!(p, "<h4>Réponses</h4><table><tbody>");
        let mut codes: Vec<&String> = reponses.keys().collect();
        codes.sort_unstable();
        for code in codes {
            let classe = if code.starts_with('2') || code.starts_with('3') {
                "ok"
            } else {
                "ko"
            };
            let _ = write!(
                p,
                "<tr><td><span class=\"code {}\">{}</span></td><td>{}</td></tr>",
                classe,
                h(code),
                h(reponses[code]["description"].as_str().unwrap_or(""))
            );
        }
        let _ = write!(p, "</tbody></table>");
    }

    let _ = write!(
        p,
        "<h4>Exemple</h4><pre><code>curl '{}'</code></pre></article>",
        h(&exemple(chemin, op, base))
    );
}

fn exemple(chemin: &str, op: &Value, base: &str) -> String {
    let params = op["parameters"].as_array().cloned().unwrap_or_default();
    let mut url = format!("{base}{chemin}");
    for x in params.iter().filter(|x| x["in"] == "path") {
        let Some(nom) = x["name"].as_str() else {
            continue;
        };
        let valeur = valeur_exemple(&x["example"]).unwrap_or_else(|| {
            if nom == "id" {
                "Z4ogLhlkS2CeUdq0bZ0YFw".to_string()
            } else {
                nom.to_string()
            }
        });
        url = url.replace(&format!("{{{nom}}}"), &valeur);
    }

    let query: Vec<String> = params
        .iter()
        .filter(|x| x["in"] == "query")
        .filter_map(|x| {
            let v = valeur_exemple(&x["example"])?;
            Some(format!("{}={}", x["name"].as_str()?, urlencode(&v)))
        })
        .take(2)
        .collect();
    if query.is_empty() {
        url
    } else {
        format!("{url}?{}", query.join("&"))
    }
}

fn valeur_exemple(v: &Value) -> Option<String> {
    v.as_str()
        .map(str::to_string)
        .or_else(|| v.as_i64().map(|n| n.to_string()))
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "%20".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn type_lisible(schema: &Value) -> String {
    match schema["type"].as_str() {
        Some("array") => format!(
            "{} (répétable)",
            schema["items"]["type"].as_str().unwrap_or("string")
        ),
        Some(t) => t.to_string(),
        None => "—".to_string(),
    }
}

fn contraintes(schema: &Value) -> Vec<String> {
    let mut v = Vec::new();

    if let Some(admises) = schema["enum"].as_array() {
        let liste: Vec<&str> = admises.iter().filter_map(Value::as_str).collect();
        if !liste.is_empty() {
            v.push(liste.join(" · "));
        }
    }

    match (schema["minimum"].as_i64(), schema["maximum"].as_i64()) {
        (Some(min), Some(max)) => v.push(format!("de {min} à {max}")),
        (Some(min), None) => v.push(format!("au moins {min}")),
        (None, Some(max)) => v.push(format!("au plus {max}")),
        (None, None) => {}
    }

    if schema["minLength"].as_i64().is_some_and(|n| n > 0) {
        v.push("non vide".to_string());
    }

    if let Some(d) = valeur_exemple(&schema["default"]) {
        v.push(format!("défaut {d}"));
    }

    v
}

fn markdown_leger(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 64);
    for para in h(s).split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        match para.strip_prefix("## ") {
            Some(titre) => {
                let _ = write!(out, "<h3 class=\"sect\">{}</h3>", enrichir(titre));
            }
            None => {
                let _ = write!(out, "<p>{}</p>", enrichir(para));
            }
        }
    }
    out
}

fn enrichir(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let (mut gras, mut code) = (false, false);
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                out.push_str(if code { "</code>" } else { "<code>" });
                code = !code;
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                out.push_str(if gras { "</strong>" } else { "<strong>" });
                gras = !gras;
            }
            _ => out.push(c),
        }
    }
    if code {
        out.push_str("</code>");
    }
    if gras {
        out.push_str("</strong>");
    }
    out
}

const STYLE: &str = r#"
:root{--fg:#1a1a1a;--fg2:#555;--bd:#e2e2e2;--bg:#fff;--acc:#0b57d0;--code:#f6f6f7}
@media(prefers-color-scheme:dark){:root{--fg:#e8e8e8;--fg2:#a0a0a0;--bd:#333;--bg:#141416;--acc:#7aa7ff;--code:#1e1e21}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);
 font:16px/1.6 system-ui,-apple-system,"Segoe UI",sans-serif;
 display:grid;grid-template-columns:260px minmax(0,1fr);gap:0}
header{grid-column:1/-1;padding:2rem 2rem 1.25rem;border-bottom:1px solid var(--bd)}
h1{margin:0 0 .5rem;font-size:1.6rem;font-weight:600}
h1 .v{font-size:.8rem;color:var(--fg2);font-weight:400}
.intro{max-width:70ch;color:var(--fg2)}
.intro p{margin:.4rem 0}
.liens{margin:1rem 0 0;font-size:.9rem}
.sect{font-size:.95rem;margin:1.2rem 0 .3rem;color:var(--fg)}
a{color:var(--acc);text-decoration:none}
a:hover{text-decoration:underline}
nav{padding:1.5rem 1rem;border-right:1px solid var(--bd);position:sticky;top:0;align-self:start;max-height:100vh;overflow:auto}
nav ul{list-style:none;margin:0;padding:0}
nav>ul>li{margin-bottom:1rem}
nav>ul>li>a{font-weight:600;color:var(--fg);font-size:.85rem;text-transform:uppercase;letter-spacing:.04em}
nav ul ul{margin:.35rem 0 0 0}
nav ul ul a{font-size:.85rem;color:var(--fg2)}
main{padding:2rem;max-width:80ch}
section{margin-bottom:3rem}
h2{font-size:1.25rem;padding-bottom:.4rem;border-bottom:1px solid var(--bd)}
article{margin:2rem 0;padding-top:.5rem}
h3{font-size:1rem;margin:0 0 .35rem;font-weight:600}
h3 code{font-size:1rem}
h4{font-size:.78rem;text-transform:uppercase;letter-spacing:.05em;color:var(--fg2);margin:1.4rem 0 .4rem}
.m{background:var(--acc);color:#fff;font-size:.7rem;padding:.15rem .4rem;border-radius:3px;vertical-align:.1em}
.resume{margin:.2rem 0 .6rem}
.desc{color:var(--fg2);font-size:.94rem}
.desc p{margin:.4rem 0}
code{background:var(--code);padding:.1rem .3rem;border-radius:3px;
 font:.88em ui-monospace,SFMono-Regular,Menlo,monospace}
pre{background:var(--code);padding:.8rem 1rem;border-radius:6px;overflow-x:auto}
pre code{background:none;padding:0}
table{border-collapse:collapse;width:100%;font-size:.9rem;margin:.3rem 0}
th{text-align:left;font-size:.72rem;text-transform:uppercase;letter-spacing:.05em;color:var(--fg2);font-weight:600}
th,td{padding:.45rem .6rem .45rem 0;border-bottom:1px solid var(--bd);vertical-align:top}
td.t{color:var(--fg2);font-size:.85rem}
.contr{display:block;font-size:.78rem;color:var(--fg2);opacity:.85;margin-top:.15rem}
.req{font-size:.7rem;color:#c0392b}
.code{display:inline-block;min-width:2.4rem;text-align:center;padding:.1rem .35rem;border-radius:3px;font-size:.8rem;font-weight:600}
.code.ok{background:#e6f4ea;color:#137333}
.code.ko{background:#fce8e6;color:#c5221f}
@media(prefers-color-scheme:dark){.code.ok{background:#0f2e1a;color:#7ee2a0}.code.ko{background:#3a1512;color:#ff9b93}}
footer{grid-column:1/-1;padding:1.5rem 2rem;border-top:1px solid var(--bd);color:var(--fg2);font-size:.85rem}
@media(max-width:820px){body{grid-template-columns:1fr}nav{position:static;border-right:none;border-bottom:1px solid var(--bd);max-height:none}}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::store_test;

    fn page() -> String {
        let s = store_test();
        let doc = crate::openapi::document(&s);
        rendre(&doc, "https://api-equides.org")
    }

    #[test]
    fn la_page_se_rend_depuis_l_openapi() {
        let html = page();

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("API Équidés"));
        for chemin in ["/v1/equides", "/v1/search", "/v1/equides/{id}/pedigree"] {
            assert!(
                html.contains(&h(chemin)),
                "chemin absent de la page : {chemin}"
            );
        }
        assert!(
            !html.contains("http://"),
            "la page ne doit charger aucun tiers"
        );
        assert!(
            !html.contains("<script"),
            "la page doit fonctionner sans JavaScript"
        );
    }

    #[test]
    fn chaque_type_d_erreur_a_son_ancre_dans_la_page() {
        let html = page();
        assert!(html.contains("id=\"erreurs\""), "section Erreurs absente");
        for t in crate::api::erreur::CATALOGUE {
            assert!(
                html.contains(&format!("id=\"{}\"", t.ancre())),
                "ancre absente pour {} : le champ `type` renverrait dans le vide",
                t.uri
            );
            assert!(html.contains(&h(t.titre)), "titre absent pour {}", t.uri);
        }
    }

    #[test]
    fn les_exemples_sont_executables() {
        let html = page();
        for gabarit in ["{id}", "{dimension}"] {
            assert!(
                !html.contains(&format!(
                    "curl 'https://api-equides.org/v1/referentiels/{gabarit}"
                )),
                "gabarit {gabarit} laissé tel quel dans un exemple"
            );
        }
        assert!(
            html.contains("curl 'https://api-equides.org/v1/referentiels/races'"),
            "l'exemple de référentiel doit porter une dimension réelle"
        );
    }

    #[test]
    fn les_bornes_des_parametres_sont_affichees() {
        let html = page();
        assert!(
            html.contains(&format!("de 1 à {}", crate::query::LIMITE_MAX)),
            "les bornes de `limite` doivent figurer sur la page"
        );
        assert!(
            html.contains(&format!("défaut {}", crate::query::LIMITE_DEFAUT)),
            "la valeur par défaut de `limite` doit figurer sur la page"
        );
        assert!(html.contains("resume · complet"));
    }

    #[test]
    fn les_sections_du_chapeau_sont_des_titres() {
        let html = page();
        assert!(
            !html.contains("## "),
            "un titre Markdown est resté littéral dans la page"
        );
        assert!(
            html.contains("<h3 class=\"sect\">Limites</h3>"),
            "la section « Limites » doit être rendue en titre"
        );
    }

    #[test]
    fn le_chapeau_reste_court() {
        let s = store_test();
        let doc = crate::openapi::document(&s);
        let chapeau = doc["info"]["description"].as_str().unwrap();
        assert!(
            chapeau.len() < 1_500,
            "chapeau de {} caractères : les explications vont dans les guides",
            chapeau.len()
        );
    }

    #[test]
    fn echappement_html() {
        assert_eq!(h("<a>&\"x\""), "&lt;a&gt;&amp;&quot;x&quot;");
    }

    #[test]
    fn markdown_leger_traite_code_et_gras() {
        let r = markdown_leger("un `code` et du **gras**");
        assert!(r.contains("<code>code</code>"));
        assert!(r.contains("<strong>gras</strong>"));
    }

    #[test]
    fn markdown_leger_neutralise_le_html_injecte() {
        let r = markdown_leger("<script>alert(1)</script>");
        assert!(!r.contains("<script"));
        assert!(r.contains("&lt;script&gt;"));
    }

    #[test]
    fn contraintes_lisibles() {
        use serde_json::json;
        assert_eq!(
            contraintes(&json!({"type": "integer", "minimum": 1, "maximum": 100, "default": 20})),
            ["de 1 à 100", "défaut 20"]
        );
        assert_eq!(
            contraintes(&json!({"type": "string", "enum": ["a", "b"], "default": "a"})),
            ["a · b", "défaut a"]
        );
        assert_eq!(
            contraintes(&json!({"type": "string", "minLength": 1})),
            ["non vide"]
        );
        assert!(contraintes(&json!({"type": "string"})).is_empty());
    }
}
