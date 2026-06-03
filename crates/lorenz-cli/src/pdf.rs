//! Escritor de PDF propio y minimo (RF-ECO-024).
//!
//! Pasa un documento Markdown (el de `documentar`) a un PDF de texto: A4, tipografias estandar del
//! formato (Helvetica y Courier, sin incrustar nada), encabezados, parrafos con ajuste de linea,
//! listas, tablas y bloques de codigo en monoespaciada, y numero de pagina. Los caracteres fuera de
//! Latin-1 se sustituyen por equivalentes simples.

const ANCHO: f32 = 595.0;
const ALTO: f32 = 842.0;
const MARGEN: f32 = 50.0;
const PIE: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fuente {
    Normal,
    Negrita,
    Mono,
}

impl Fuente {
    fn nombre(self) -> &'static str {
        match self {
            Fuente::Normal => "F1",
            Fuente::Negrita => "F2",
            Fuente::Mono => "F3",
        }
    }

    /// Ancho medio de un caracter, como fraccion del cuerpo.
    fn ancho(self) -> f32 {
        match self {
            Fuente::Normal => 0.5,
            Fuente::Negrita => 0.55,
            Fuente::Mono => 0.6,
        }
    }
}

struct Linea {
    texto: String,
    fuente: Fuente,
    cuerpo: f32,
    sangria: f32,
    antes: f32,
}

/// Codifica en Latin-1 (la codificacion `WinAnsi` coincide en ese rango) y escapa lo que el formato
/// reserva.
fn codificar(t: &str) -> Vec<u8> {
    let mut o = Vec::new();
    for c in t.chars() {
        let c = match c {
            '\u{2192}' => {
                o.extend_from_slice(b"->");
                continue;
            }
            '\u{2190}' => {
                o.extend_from_slice(b"<-");
                continue;
            }
            '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2026}' => {
                o.extend_from_slice(b"...");
                continue;
            }
            '\u{2264}' => {
                o.extend_from_slice(b"<=");
                continue;
            }
            '\u{2265}' => {
                o.extend_from_slice(b">=");
                continue;
            }
            '\t' => ' ',
            c => c,
        };
        let n = c as u32;
        if n == 0x28 || n == 0x29 || n == 0x5C {
            o.push(b'\\');
            o.push(n as u8);
        } else if (0x20..=0x7E).contains(&n) || (0xA0..=0xFF).contains(&n) {
            o.push(n as u8);
        } else {
            o.push(b'?');
        }
    }
    o
}

/// Quita el adorno de Markdown dentro de una linea de texto.
fn limpiar(t: &str) -> String {
    let mut s = t.replace("**", "").replace("__", "").replace('`', "");
    while let (Some(a), Some(b)) = (s.find("]("), s.find('[')) {
        if b >= a {
            break;
        }
        let Some(fin) = s[a..].find(')') else { break };
        let texto = s[b + 1..a].to_string();
        s.replace_range(b..a + fin + 1, &texto);
    }
    s
}

/// Parte `texto` en lineas de a lo sumo `maximo` caracteres, cortando en espacios.
fn ajustar(texto: &str, maximo: usize) -> Vec<String> {
    let maximo = maximo.max(8);
    let mut lineas = Vec::new();
    let mut actual = String::new();
    for palabra in texto.split_whitespace() {
        let mut palabra = palabra.to_string();
        while palabra.chars().count() > maximo {
            if !actual.is_empty() {
                lineas.push(std::mem::take(&mut actual));
            }
            let corte: String = palabra.chars().take(maximo).collect();
            palabra = palabra.chars().skip(maximo).collect();
            lineas.push(corte);
        }
        let largo = actual.chars().count();
        if largo > 0 && largo + 1 + palabra.chars().count() > maximo {
            lineas.push(std::mem::take(&mut actual));
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(&palabra);
    }
    if !actual.is_empty() {
        lineas.push(actual);
    }
    lineas
}

fn maquetar(md: &str) -> Vec<Linea> {
    let util = ANCHO - 2.0 * MARGEN;
    let mut v: Vec<Linea> = Vec::new();
    let mut en_codigo = false;
    let mut parrafo = String::new();
    fn vaciar(parrafo: &mut String, v: &mut Vec<Linea>, util: f32) {
        if parrafo.is_empty() {
            return;
        }
        let max = (util / (10.0 * Fuente::Normal.ancho())) as usize;
        for (i, l) in ajustar(&limpiar(parrafo), max).into_iter().enumerate() {
            v.push(Linea {
                texto: l,
                fuente: Fuente::Normal,
                cuerpo: 10.0,
                sangria: 0.0,
                antes: if i == 0 { 5.0 } else { 0.0 },
            });
        }
        parrafo.clear();
    }
    for crudo in md.lines() {
        let l = crudo.trim_end();
        if l.trim_start().starts_with("```") {
            vaciar(&mut parrafo, &mut v, util);
            en_codigo = !en_codigo;
            continue;
        }
        if en_codigo || l.trim_start().starts_with('|') {
            vaciar(&mut parrafo, &mut v, util);
            // Las filas de separacion de las tablas no aportan nada al papel.
            if !en_codigo && l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                continue;
            }
            let max = (util / (8.0 * Fuente::Mono.ancho())) as usize;
            let texto = if en_codigo { l.to_string() } else { limpiar(l) };
            let trozos: Vec<String> = if texto.chars().count() <= max {
                vec![texto]
            } else {
                texto
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(max)
                    .map(|c| c.iter().collect())
                    .collect()
            };
            for t in trozos {
                v.push(Linea {
                    texto: t,
                    fuente: Fuente::Mono,
                    cuerpo: 8.0,
                    sangria: 0.0,
                    antes: 0.0,
                });
            }
            continue;
        }
        let nivel = l.chars().take_while(|&c| c == '#').count();
        if (1..=4).contains(&nivel) && l[nivel..].starts_with(' ') {
            vaciar(&mut parrafo, &mut v, util);
            let (cuerpo, antes) = match nivel {
                1 => (20.0, 14.0),
                2 => (15.0, 12.0),
                3 => (12.0, 9.0),
                _ => (10.5, 7.0),
            };
            let max = (util / (cuerpo * Fuente::Negrita.ancho())) as usize;
            for (i, t) in ajustar(&limpiar(l[nivel..].trim()), max)
                .into_iter()
                .enumerate()
            {
                v.push(Linea {
                    texto: t,
                    fuente: Fuente::Negrita,
                    cuerpo,
                    sangria: 0.0,
                    antes: if i == 0 { antes } else { 0.0 },
                });
            }
            continue;
        }
        if let Some(item) = l
            .trim_start()
            .strip_prefix("- ")
            .or_else(|| l.trim_start().strip_prefix("* "))
        {
            vaciar(&mut parrafo, &mut v, util);
            let max = ((util - 14.0) / (10.0 * Fuente::Normal.ancho())) as usize;
            for (i, t) in ajustar(&limpiar(item), max).into_iter().enumerate() {
                let texto = if i == 0 {
                    format!("- {t}")
                } else {
                    format!("  {t}")
                };
                v.push(Linea {
                    texto,
                    fuente: Fuente::Normal,
                    cuerpo: 10.0,
                    sangria: 8.0,
                    antes: if i == 0 { 2.0 } else { 0.0 },
                });
            }
            continue;
        }
        if l.trim().is_empty() {
            vaciar(&mut parrafo, &mut v, util);
            continue;
        }
        if !parrafo.is_empty() {
            parrafo.push(' ');
        }
        parrafo.push_str(l.trim());
    }
    vaciar(&mut parrafo, &mut v, util);
    v
}

fn numero(x: f32) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Paginas: el contenido de cada una como flujo de operadores del PDF.
fn paginar(lineas: &[Linea], pie: &str) -> Vec<Vec<u8>> {
    let mut paginas: Vec<Vec<u8>> = Vec::new();
    let mut flujo: Vec<u8> = Vec::new();
    let mut y = ALTO - MARGEN;
    let nueva = |flujo: &mut Vec<u8>, paginas: &mut Vec<Vec<u8>>| {
        let n = paginas.len() + 1;
        let mut f = std::mem::take(flujo);
        f.extend_from_slice(
            format!("BT /F1 8 Tf {} {} Td (", numero(MARGEN), numero(PIE - 10.0)).as_bytes(),
        );
        f.extend_from_slice(&codificar(&format!("{pie}  -  pagina {n}")));
        f.extend_from_slice(b") Tj ET\n");
        paginas.push(f);
    };
    for l in lineas {
        let alto = l.cuerpo * 1.25;
        if y - l.antes - alto < PIE + 10.0 {
            nueva(&mut flujo, &mut paginas);
            y = ALTO - MARGEN;
        }
        y -= l.antes + alto;
        flujo.extend_from_slice(
            format!(
                "BT /{} {} Tf {} {} Td (",
                l.fuente.nombre(),
                numero(l.cuerpo),
                numero(MARGEN + l.sangria),
                numero(y)
            )
            .as_bytes(),
        );
        flujo.extend_from_slice(&codificar(&l.texto));
        flujo.extend_from_slice(b") Tj ET\n");
    }
    nueva(&mut flujo, &mut paginas);
    paginas
}

/// Convierte el Markdown en un PDF. `titulo` va en los metadatos y en el pie de pagina.
pub fn markdown_a_pdf(md: &str, titulo: &str) -> Vec<u8> {
    let paginas = paginar(&maquetar(md), titulo);
    let n = paginas.len();
    // Objetos: 1 catalogo, 2 paginas, 3-5 fuentes, 6 info, luego (pagina, contenido) por pagina.
    let mut objetos: Vec<Vec<u8>> = Vec::new();
    objetos.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let hijos: Vec<String> = (0..n).map(|i| format!("{} 0 R", 7 + 2 * i)).collect();
    objetos.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", hijos.join(" ")).into_bytes());
    for base in ["Helvetica", "Helvetica-Bold", "Courier"] {
        objetos.push(
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>"
            )
            .into_bytes(),
        );
    }
    let mut info = b"<< /Producer (Lorenz) /Title (".to_vec();
    info.extend_from_slice(&codificar(titulo));
    info.extend_from_slice(b") /Subject (");
    info.extend_from_slice(&codificar(&lorenz_arch::marca_generado(
        "lorenz exportar pdf",
    )));
    info.extend_from_slice(b") >>");
    objetos.push(info);
    for (i, flujo) in paginas.iter().enumerate() {
        objetos.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {} 0 R >>",
                numero(ANCHO), numero(ALTO), 8 + 2 * i
            )
            .into_bytes(),
        );
        let mut o = format!("<< /Length {} >>\nstream\n", flujo.len()).into_bytes();
        o.extend_from_slice(flujo);
        o.extend_from_slice(b"endstream");
        objetos.push(o);
    }
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut posiciones = Vec::new();
    for (i, o) in objetos.iter().enumerate() {
        posiciones.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(o);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objetos.len() + 1).as_bytes(),
    );
    for p in posiciones {
        pdf.extend_from_slice(format!("{p:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objetos.len() + 1
        )
        .as_bytes(),
    );
    pdf
}
