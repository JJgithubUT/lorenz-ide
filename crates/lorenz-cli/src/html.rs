//! Convierte el Markdown que produce `documentar` en una pagina HTML autonoma (RF-ECO-025).
//!
//! Solo entiende lo que `documentar` genera: titulos `#`, tablas, bloques de codigo con ```, listas
//! con `-` y parrafos, con `codigo` y **negrita** en linea. Todo el texto se escapa.

pub fn escapar(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            c => s.push(c),
        }
    }
    s
}

/// `codigo` y **negrita** dentro de una linea ya sin formato de bloque.
fn en_linea(t: &str) -> String {
    let mut sal = String::new();
    let mut resto = t;
    while !resto.is_empty() {
        let (marca, cierre) = if resto.starts_with('`') {
            ("`", "code")
        } else if resto.starts_with("**") {
            ("**", "strong")
        } else {
            let sig = resto
                .char_indices()
                .skip(1)
                .find(|(_, c)| *c == '`' || *c == '*')
                .map_or(resto.len(), |(i, _)| i);
            sal.push_str(&escapar(&resto[..sig]));
            resto = &resto[sig..];
            continue;
        };
        let cuerpo = &resto[marca.len()..];
        match cuerpo.find(marca) {
            Some(f) => {
                sal.push_str(&format!("<{cierre}>{}</{cierre}>", escapar(&cuerpo[..f])));
                resto = &cuerpo[f + marca.len()..];
            }
            None => {
                sal.push_str(&escapar(marca));
                resto = cuerpo;
            }
        }
    }
    sal
}

fn celdas(l: &str) -> Vec<String> {
    let l = l.trim().trim_start_matches('|').trim_end_matches('|');
    let mut v = Vec::new();
    let mut actual = String::new();
    let mut it = l.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' if it.peek() == Some(&'|') => {
                actual.push('|');
                it.next();
            }
            '|' => v.push(std::mem::take(&mut actual).trim().to_string()),
            c => actual.push(c),
        }
    }
    v.push(actual.trim().to_string());
    v
}

fn es_separador(l: &str) -> bool {
    let l = l.trim();
    l.starts_with('|') && l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

pub fn markdown_a_cuerpo(md: &str) -> String {
    let lineas: Vec<&str> = md.lines().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < lineas.len() {
        let l = lineas[i];
        if l.trim().is_empty() {
            i += 1;
        } else if l.starts_with("```") {
            i += 1;
            o.push_str("<pre><code>");
            while i < lineas.len() && !lineas[i].starts_with("```") {
                o.push_str(&escapar(lineas[i]));
                o.push('\n');
                i += 1;
            }
            o.push_str("</code></pre>\n");
            i += 1;
        } else if l.starts_with('#') {
            let n = l.chars().take_while(|c| *c == '#').count().min(6);
            o.push_str(&format!("<h{n}>{}</h{n}>\n", en_linea(l[n..].trim())));
            i += 1;
        } else if l.starts_with('|') {
            let mut fin = i;
            while fin < lineas.len() && lineas[fin].starts_with('|') {
                fin += 1;
            }
            let bloque = &lineas[i..fin];
            o.push_str("<table>\n");
            let mut cuerpo = false;
            for (k, fila) in bloque.iter().enumerate() {
                if es_separador(fila) {
                    continue;
                }
                let cab = k == 0 && bloque.get(1).is_some_and(|s| es_separador(s));
                let etiqueta = if cab { "th" } else { "td" };
                if cab {
                    o.push_str("<thead>");
                } else if !cuerpo {
                    o.push_str("<tbody>\n");
                    cuerpo = true;
                }
                o.push_str("<tr>");
                for c in celdas(fila) {
                    o.push_str(&format!("<{etiqueta}>{}</{etiqueta}>", en_linea(&c)));
                }
                o.push_str("</tr>");
                if cab {
                    o.push_str("</thead>");
                }
                o.push('\n');
            }
            if cuerpo {
                o.push_str("</tbody>\n");
            }
            o.push_str("</table>\n");
            i = fin;
        } else if l.starts_with("- ") {
            o.push_str("<ul>\n");
            while i < lineas.len() && lineas[i].starts_with("- ") {
                o.push_str(&format!("<li>{}</li>\n", en_linea(&lineas[i][2..])));
                i += 1;
            }
            o.push_str("</ul>\n");
        } else {
            let mut p = Vec::new();
            while i < lineas.len()
                && !lineas[i].trim().is_empty()
                && !lineas[i].starts_with(['#', '|'])
                && !lineas[i].starts_with("```")
                && !lineas[i].starts_with("- ")
            {
                p.push(lineas[i].trim());
                i += 1;
            }
            o.push_str(&format!("<p>{}</p>\n", en_linea(&p.join(" "))));
        }
    }
    o
}

const ESTILO: &str = "body{font-family:system-ui,sans-serif;max-width:60rem;margin:2rem auto;padding:0 1rem;line-height:1.5;color:#1a1a1a}\
table{border-collapse:collapse;margin:1rem 0;font-size:.92rem}th,td{border:1px solid #c8c8c8;padding:.3rem .6rem;text-align:left;vertical-align:top}\
th{background:#f0f0f0}pre{background:#f6f6f6;padding:.6rem;overflow-x:auto;border-radius:4px}code{font-family:ui-monospace,monospace;font-size:.92em}\
h2{border-bottom:1px solid #ddd;padding-bottom:.2rem;margin-top:2rem}";

/// Pagina completa; el titulo es el primer encabezado del documento.
pub fn markdown_a_html(md: &str) -> String {
    let titulo = md
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("Documentacion");
    format!(
        "<!DOCTYPE html>\n<html lang=\"es\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{ESTILO}</style>\n</head>\n<body>\n{}</body>\n</html>\n",
        escapar(titulo),
        markdown_a_cuerpo(md)
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn escapa_y_marca_en_linea() {
        assert_eq!(
            en_linea("a <b> `x<y` **n** z"),
            "a &lt;b&gt; <code>x&lt;y</code> <strong>n</strong> z"
        );
        assert_eq!(en_linea("sin cerrar `x"), "sin cerrar `x");
    }

    #[test]
    fn tabla_con_barra_escapada() {
        let h = markdown_a_cuerpo("| A | B |\n| --- | --- |\n| `x \\| y` | 2 |\n");
        assert!(
            h.contains("<th>A</th>") && h.contains("<td><code>x | y</code></td>"),
            "{h}"
        );
    }

    #[test]
    fn bloques_y_listas() {
        let h = markdown_a_cuerpo("# T\n\nuno\ndos\n\n- a\n- b\n\n```text\n<x>\n```\n");
        assert!(h.contains("<h1>T</h1>") && h.contains("<p>uno dos</p>"));
        assert!(
            h.contains("<li>b</li>") && h.contains("<pre><code>&lt;x&gt;\n</code></pre>"),
            "{h}"
        );
    }
}
