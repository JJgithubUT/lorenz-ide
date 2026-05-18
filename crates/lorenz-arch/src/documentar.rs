//! Generador de la documentacion de una arquitectura (RF-ECO-017 a RF-ECO-023, RF-ARQ-036).
//!
//! La documentacion sale solo de la descripcion: no hay nada que escribir a mano ni que mantener
//! aparte. El resultado es Markdown, determinista (la misma descripcion da siempre el mismo texto)
//! y sin fechas, para que se pueda versionar y comparar.
//!
//! Los comentarios del TOML que van justo encima de una tabla pasan a la documentacion como texto
//! del elemento al que describen.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::bits::{Codificacion, Pieza};
use crate::modelo::{
    Acceso, Ciclos, Instruccion, Maquina, RolRegistro, SentidoPila, TipoDispositivo, TipoOperando,
};

/// Comentarios del archivo, por elemento.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Comentarios {
    /// Comentario que abre el archivo, antes de `[maquina]`.
    pub general: String,
    /// Clave `(tipo, nombre)`: `("instruccion", "ADD")`, `("registro", "A")`...
    por_elemento: BTreeMap<(String, String), String>,
}

impl Comentarios {
    /// Comentario de un elemento, vacio si no tiene.
    pub fn de(&self, tipo: &str, nombre: &str) -> &str {
        self.por_elemento
            .get(&(tipo.to_string(), nombre.to_string()))
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Lee los comentarios del texto TOML de una descripcion.
    ///
    /// Un comentario pertenece a una tabla si ocupa las lineas inmediatamente anteriores a su
    /// cabecera, sin lineas en blanco de por medio; el nombre del elemento se toma de su primera
    /// clave `nombre` o `mnemonico`. En `[flags]` un comentario pertenece a la bandera de la linea
    /// siguiente. El bloque que abre el archivo describe la maquina entera.
    pub fn leer(texto: &str) -> Self {
        let mut c = Comentarios::default();
        let lineas: Vec<&str> = texto.lines().collect();
        let mut bloque: Vec<String> = Vec::new();
        let mut primera_tabla_vista = false;
        let mut en_flags = false;
        let mut pendiente: Option<(String, Vec<String>)> = None;

        for linea in &lineas {
            let t = linea.trim();
            if let Some(resto) = t.strip_prefix('#') {
                bloque.push(
                    resto
                        .strip_prefix(' ')
                        .unwrap_or(resto)
                        .trim_end()
                        .to_string(),
                );
                continue;
            }
            if t.is_empty() {
                // Un bloque separado de lo siguiente por una linea en blanco no describe nada,
                // salvo que sea el que abre el archivo.
                if !primera_tabla_vista && c.general.is_empty() && !bloque.is_empty() {
                    c.general = bloque.join("\n");
                }
                bloque.clear();
                continue;
            }
            if let Some(tabla) = cabecera(t) {
                if !primera_tabla_vista {
                    primera_tabla_vista = true;
                    if c.general.is_empty() && !bloque.is_empty() {
                        c.general = bloque.join("\n");
                    }
                }
                en_flags = tabla == "flags";
                pendiente = (!bloque.is_empty() && !en_flags && tabla != "maquina")
                    .then(|| (tabla.to_string(), std::mem::take(&mut bloque)));
                bloque.clear();
                continue;
            }
            if en_flags {
                if let Some(nombre) = t.split('=').next().map(str::trim) {
                    if !bloque.is_empty() && !nombre.is_empty() {
                        c.por_elemento
                            .insert(("bandera".into(), nombre.to_string()), bloque.join("\n"));
                    }
                }
            } else if let Some((tipo, comentario)) = pendiente.take() {
                if let Some(nombre) = valor_de(t, "mnemonico").or_else(|| valor_de(t, "nombre")) {
                    c.por_elemento.insert((tipo, nombre), comentario.join("\n"));
                } else {
                    pendiente = Some((tipo, comentario));
                }
            }
            bloque.clear();
        }
        c
    }
}

/// `[[tipo]]` o `[tipo]` -> `tipo`. Las subtablas (`[maquina.interrupciones]`) no cuentan.
fn cabecera(linea: &str) -> Option<&str> {
    let dentro = linea
        .strip_prefix("[[")
        .and_then(|l| l.strip_suffix("]]"))
        .or_else(|| linea.strip_prefix('[').and_then(|l| l.strip_suffix(']')))?;
    (!dentro.contains('.')).then_some(dentro.trim())
}

fn valor_de(linea: &str, clave: &str) -> Option<String> {
    let resto = linea.strip_prefix(clave)?.trim_start().strip_prefix('=')?;
    let resto = resto.trim().strip_prefix('"')?;
    Some(resto.split('"').next()?.to_string())
}

/// Genera la documentacion completa en Markdown.
///
/// `comentarios` sale de [`Comentarios::leer`]; sin el texto original se usa `Comentarios::default()`
/// y la documentacion solo lleva lo declarado en las claves `descripcion`.
pub fn documentar(m: &Maquina, comentarios: &Comentarios) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# {}\n", m.nombre);
    if !comentarios.general.is_empty() {
        let _ = writeln!(s, "{}\n", comentarios.general);
    }
    resumen(&mut s, m);
    registros(&mut s, m, comentarios);
    memoria(&mut s, m, comentarios);
    dispositivos(&mut s, m, comentarios);
    interrupciones(&mut s, m);
    tabla_de_codigos(&mut s, m);
    detalle(&mut s, m, comentarios);
    s
}

fn resumen(s: &mut String, m: &Maquina) {
    let _ = writeln!(s, "## Resumen\n");
    let _ = writeln!(s, "| Propiedad | Valor |\n| --- | --- |");
    let _ = writeln!(s, "| Version del formato | {} |", m.formato);
    let _ = writeln!(s, "| Ancho de palabra | {} bits |", m.bits_palabra);
    let _ = writeln!(s, "| Ancho de direccion | {} bits |", m.bits_direccion);
    let _ = writeln!(
        s,
        "| Orden de los bytes | {} |",
        match m.endianness {
            crate::modelo::Endianness::Little => "little endian",
            crate::modelo::Endianness::Big => "big endian",
        }
    );
    let _ = writeln!(s, "| Direccion de arranque | {} |", hex(m.arranque, m));
    let _ = writeln!(
        s,
        "| Pila | {} |",
        match m.pila {
            SentidoPila::Descendente => "crece hacia direcciones menores",
            SentidoPila::Ascendente => "crece hacia direcciones mayores",
        }
    );
    let _ = writeln!(s, "| Instrucciones | {} |\n", m.instrucciones.len());
}

fn hex(v: u64, m: &Maquina) -> String {
    let digitos = (m.bits_direccion as usize).div_ceil(4);
    format!("0x{v:0digitos$X}")
}

fn rol(r: RolRegistro) -> &'static str {
    match r {
        RolRegistro::General => "general",
        RolRegistro::ContadorPrograma => "contador de programa",
        RolRegistro::PunteroPila => "puntero de pila",
        RolRegistro::Banderas => "banderas",
        RolRegistro::Indirecto => "indirecto",
        RolRegistro::Fijo => "fijo (solo lectura)",
    }
}

/// Escapa lo que romperia una celda de tabla.
fn celda(t: &str) -> String {
    t.replace('|', "\\|").replace('\n', " ")
}

/// RF-ECO-021: registros y banderas.
fn registros(s: &mut String, m: &Maquina, c: &Comentarios) {
    let _ = writeln!(s, "## Registros y banderas\n");
    let _ = writeln!(
        s,
        "| Registro | Bits | Rol | Valor inicial | Notas |\n| --- | --- | --- | --- | --- |"
    );
    for r in &m.registros {
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | 0x{:X} | {} |",
            r.nombre,
            r.bits,
            rol(r.rol),
            r.inicial,
            celda(c.de("registro", &r.nombre))
        );
    }
    let _ = writeln!(s);
    if !m.pares.is_empty() {
        let _ = writeln!(
            s,
            "| Par | Registro alto | Registro bajo | Bits | Notas |\n| --- | --- | --- | --- | --- |"
        );
        for p in &m.pares {
            let bits: u32 = [&p.alto, &p.bajo]
                .iter()
                .filter_map(|n| m.registro(n))
                .map(|r| r.bits)
                .sum();
            let _ = writeln!(
                s,
                "| `{}` | `{}` | `{}` | {} | {} |",
                p.nombre,
                p.alto,
                p.bajo,
                bits,
                celda(c.de("par", &p.nombre))
            );
        }
        let _ = writeln!(s);
    }
    if !m.banderas.is_empty() {
        let _ = writeln!(s, "| Bandera | Bit | Significado |\n| --- | --- | --- |");
        let mut banderas: Vec<_> = m.banderas.iter().collect();
        banderas.sort_by_key(|b| b.bit);
        for b in banderas {
            let texto = if c.de("bandera", &b.nombre).is_empty() {
                b.descripcion.clone()
            } else {
                c.de("bandera", &b.nombre).to_string()
            };
            let _ = writeln!(s, "| `{}` | {} | {} |", b.nombre, b.bit, celda(&texto));
        }
        let _ = writeln!(s);
    }
}

fn acceso(a: Acceso) -> &'static str {
    match (a.lectura, a.escritura) {
        (true, true) => "lectura y escritura",
        (true, false) => "solo lectura",
        (false, true) => "solo escritura",
        (false, false) => "ninguno",
    }
}

/// RF-ECO-020: mapa de memoria con rangos y permisos.
fn memoria(s: &mut String, m: &Maquina, c: &Comentarios) {
    let _ = writeln!(s, "## Mapa de memoria\n");
    let _ = writeln!(
        s,
        "| Region | Desde | Hasta | Palabras | Acceso | Contenido | Notas |\n| --- | --- | --- | --- | --- | --- | --- |"
    );
    let mut regiones: Vec<_> = m.regiones.iter().collect();
    regiones.sort_by_key(|r| r.desde);
    for r in regiones {
        let contenido = match (&r.dispositivo, &r.contenido) {
            (Some(d), _) => format!("dispositivo `{d}`"),
            (None, Some(f)) => format!("precargada con `{f}`"),
            (None, None) => "memoria".to_string(),
        };
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} | {} | {} | {} |",
            r.nombre,
            hex(r.desde, m),
            hex(r.hasta, m),
            r.hasta - r.desde + 1,
            acceso(r.acceso),
            contenido,
            celda(c.de("region", &r.nombre))
        );
    }
    let _ = writeln!(s);
}

fn dispositivos(s: &mut String, m: &Maquina, c: &Comentarios) {
    if m.dispositivos.is_empty() {
        return;
    }
    let _ = writeln!(s, "## Dispositivos\n");
    let _ = writeln!(
        s,
        "| Dispositivo | Tipo | Puertos | Interrupcion | Notas |\n| --- | --- | --- | --- | --- |"
    );
    for d in &m.dispositivos {
        let tipo = match &d.tipo {
            TipoDispositivo::Pantalla {
                ancho,
                alto,
                formato,
            } => format!("pantalla {ancho}x{alto} ({formato})"),
            TipoDispositivo::Teclado => "teclado".into(),
            TipoDispositivo::Uart => "UART".into(),
            TipoDispositivo::Dma => "DMA".into(),
            TipoDispositivo::Temporizador { periodo } => {
                format!("temporizador (cada {periodo} ciclos)")
            }
            TipoDispositivo::Bloques {
                tamano_bloque,
                cantidad_bloques,
            } => format!("bloques ({cantidad_bloques} de {tamano_bloque} palabras)"),
            TipoDispositivo::Externo { orden, direccion } => match (orden, direccion) {
                (Some(o), _) => format!("externo (proceso `{o}`)"),
                (_, Some(d)) => format!("externo (TCP {d})"),
                _ => "externo".into(),
            },
            TipoDispositivo::Generico { .. } => "generico".into(),
        };
        let puertos = if d.puertos.is_empty() {
            "-".to_string()
        } else {
            d.puertos
                .iter()
                .map(|(n, p)| format!("`{n}` = 0x{p:X}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} | {} |",
            d.nombre,
            tipo,
            puertos,
            d.interrupcion
                .map(|l| l.to_string())
                .unwrap_or_else(|| "-".into()),
            celda(c.de("dispositivo", &d.nombre))
        );
    }
    let _ = writeln!(s);
}

fn interrupciones(s: &mut String, m: &Maquina) {
    let Some(i) = &m.interrupciones else {
        return;
    };
    let _ = writeln!(s, "## Interrupciones\n");
    let _ = writeln!(
        s,
        "Al arrancar estan {}.\n",
        if i.habilitadas_al_arrancar {
            "habilitadas"
        } else {
            "inhabilitadas"
        }
    );
    let _ = writeln!(s, "| Linea | Vector | Prioridad |\n| --- | --- | --- |");
    for (linea, vector) in &i.vectores {
        let _ = writeln!(
            s,
            "| {} | {} | {} |",
            linea,
            hex(*vector, m),
            i.prioridad_de(*linea)
        );
    }
    let _ = writeln!(s);
}

/// Clave de orden de una instruccion: los bits fijos de su primera palabra con los campos y el
/// relleno a cero, de modo que las instrucciones salen en orden de codigo de operacion.
fn clave_de_orden(c: &Codificacion) -> (u64, String) {
    let valor = c.palabras().first().map(|p| p.valor_fijo()).unwrap_or(0);
    (valor, String::new())
}

/// La primera palabra como patron: `0011 rd:2 ..`.
fn patron(c: &Codificacion) -> String {
    c.texto().to_string()
}

fn ciclos(c: Ciclos) -> String {
    match c {
        Ciclos::Fijo(n) => n.to_string(),
        Ciclos::SegunSalto { tomado, no_tomado } => {
            format!("{tomado} si salta, {no_tomado} si no")
        }
    }
}

fn ordenadas(m: &Maquina) -> Vec<&Instruccion> {
    let mut v: Vec<&Instruccion> = m.instrucciones.iter().collect();
    v.sort_by(|a, b| {
        clave_de_orden(&a.codificacion)
            .0
            .cmp(&clave_de_orden(&b.codificacion).0)
            .then_with(|| a.mnemonico.cmp(&b.mnemonico))
    });
    v
}

/// RF-ECO-018: tabla de codigos de operacion ordenada; RF-ECO-022: coste en ciclos.
fn tabla_de_codigos(s: &mut String, m: &Maquina) {
    let _ = writeln!(s, "## Codigos de operacion\n");
    let _ = writeln!(
        s,
        "| Codigo | Mnemonico | Sintaxis | Ciclos | Banderas |\n| --- | --- | --- | --- | --- |"
    );
    for i in ordenadas(m) {
        let afecta = if i.afecta.is_empty() {
            "-".to_string()
        } else {
            i.afecta.join(", ")
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` | `{}` | {} | {} |",
            celda(&patron(&i.codificacion)),
            i.mnemonico,
            i.sintaxis,
            ciclos(i.ciclos),
            afecta
        );
    }
    let _ = writeln!(s);
}

/// RF-ECO-019: diagrama de codificacion de una instruccion, una caja por palabra.
fn diagrama(c: &Codificacion) -> String {
    let mut s = String::new();
    for (n, palabra) in c.palabras().iter().enumerate() {
        let ancho = palabra.ancho();
        let _ = writeln!(s, "palabra {} ({} bits)", n + 1, ancho);
        let mut numeros = String::from(" ");
        let mut borde = String::from("+");
        let mut celdas = String::from("|");
        let mut bit = ancho;
        for pieza in palabra.piezas() {
            let n_bits = pieza.bits();
            let hueco = (4 * n_bits - 1) as usize;
            for k in 0..n_bits {
                let _ = write!(numeros, "{:^3} ", bit - 1 - k);
            }
            bit -= n_bits;
            borde.push_str(&"-".repeat(hueco));
            borde.push('+');
            let texto = match pieza {
                Pieza::Fijo(b) => (if *b { "1" } else { "0" }).to_string(),
                Pieza::Relleno => ".".to_string(),
                Pieza::Campo { nombre, .. } => nombre.clone(),
            };
            let texto: String = texto.chars().take(hueco).collect();
            let _ = write!(celdas, "{texto:^hueco$}|");
        }
        let _ = writeln!(s, "{}", numeros.trim_end());
        let _ = writeln!(s, "{borde}");
        let _ = writeln!(s, "{celdas}");
        let _ = writeln!(s, "{borde}");
    }
    s
}

fn operando(o: &crate::modelo::Operando) -> String {
    match &o.tipo {
        TipoOperando::Registro { permitidos } => format!(
            "`{}`: registro, uno de {}",
            o.nombre,
            permitidos
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TipoOperando::Inmediato { signo, rango } => format!(
            "`{}`: constante {} entre {} y {}",
            o.nombre,
            if *signo { "con signo" } else { "sin signo" },
            rango.0,
            rango.1
        ),
        TipoOperando::Direccion => format!("`{}`: direccion absoluta", o.nombre),
        TipoOperando::Desplazamiento => {
            format!(
                "`{}`: desplazamiento relativo al contador de programa",
                o.nombre
            )
        }
        TipoOperando::Puerto => format!("`{}`: puerto de entrada y salida", o.nombre),
    }
}

fn detalle(s: &mut String, m: &Maquina, c: &Comentarios) {
    let _ = writeln!(s, "## Instrucciones\n");
    for i in ordenadas(m) {
        let _ = writeln!(s, "### {}\n", i.mnemonico);
        let comentario = c.de("instruccion", &i.mnemonico);
        for texto in [i.descripcion.as_str(), comentario] {
            if !texto.is_empty() {
                let _ = writeln!(s, "{texto}\n");
            }
        }
        let _ = writeln!(s, "- Sintaxis: `{}`", i.sintaxis);
        let _ = writeln!(s, "- Codificacion: `{}`", patron(&i.codificacion));
        let _ = writeln!(s, "- Ciclos: {}", ciclos(i.ciclos));
        if !i.afecta.is_empty() {
            let _ = writeln!(s, "- Banderas que modifica: {}", i.afecta.join(", "));
        }
        if i.semantica.trim().is_empty() {
            let _ = writeln!(s, "- Efecto: ninguno");
        } else {
            let _ = writeln!(s, "- Efecto: `{}`", i.semantica.trim());
        }
        for o in &i.operandos {
            let _ = writeln!(s, "- Operando {}", operando(o));
        }
        let _ = writeln!(s, "\n```text\n{}```\n", diagrama(&i.codificacion));
    }
}
