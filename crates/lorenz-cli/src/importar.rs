//! Importar una arquitectura desde una tabla de instrucciones en texto (RF-ECO-015).
//!
//! La documentacion publica de maquinas como JDH-8 o Astro-8 es una tabla de instrucciones. La
//! tabla se pasa a este formato, una linea por elemento, y `lorenz importar` genera la descripcion
//! TOML y la valida antes de escribirla:
//!
//! ```text
//! maquina: jdh-8 palabra=8 direccion=16 endian=little arranque=0
//! registros: A B C D
//! pc: PC 16
//! sp: SP 16 0xEFFF
//! bandera: Z 0 resultado igual a cero
//! region: rom 0x0000 0x7FFF r
//! instruccion: LDI {rd}, {imm} ; 0010 rd:2 .. | imm:8 ; rd = imm ; 2
//! ```
//!
//! Los operandos se clasifican por su nombre: `rd`, `rs`, `rt`... son registros; `imm` es un
//! inmediato sin signo y `imms` con signo (su rango sale del ancho del campo); `dir` es una
//! direccion y `off`/`rel` un desplazamiento. Los ciclos son un numero o `tomado/no_tomado`.

use std::io::Write;

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

#[derive(Default)]
struct Tabla {
    nombre: String,
    palabra: u32,
    direccion: u32,
    endian: String,
    arranque: String,
    registros: Vec<String>,
    otros_registros: Vec<(String, u32, &'static str, Option<String>)>,
    banderas: Vec<(String, u32, String)>,
    regiones: Vec<(String, String, String, String)>,
    instrucciones: Vec<Instruccion>,
}

struct Instruccion {
    sintaxis: String,
    codigo: String,
    semantica: String,
    ciclos: String,
    afecta: Vec<String>,
    linea: usize,
}

fn comillas(t: &str) -> String {
    format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\""))
}

fn numero(t: &str) -> Option<u64> {
    let t = t.trim();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()
    } else {
        t.parse().ok()
    }
}

fn leer(texto: &str) -> Result<Tabla, String> {
    let mut t = Tabla {
        palabra: 0,
        direccion: 0,
        ..Tabla::default()
    };
    for (i, linea) in texto.lines().enumerate() {
        let n = i + 1;
        let linea = linea.split('#').next().unwrap_or("").trim();
        if linea.is_empty() {
            continue;
        }
        let err = |m: &str| format!("linea {n}: {m}");
        let (clave, resto) = linea
            .split_once(':')
            .ok_or_else(|| err("falta `clave:` al principio"))?;
        let resto = resto.trim();
        match clave.trim() {
            "maquina" => {
                let mut p = resto.split_whitespace();
                t.nombre = p.next().ok_or_else(|| err("falta el nombre"))?.to_string();
                t.endian = "little".into();
                t.arranque = "0x0000".into();
                for kv in p {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| err("se esperaba clave=valor"))?;
                    match k {
                        "palabra" => {
                            t.palabra = numero(v).ok_or_else(|| err("palabra invalida"))? as u32
                        }
                        "direccion" => {
                            t.direccion = numero(v).ok_or_else(|| err("direccion invalida"))? as u32
                        }
                        "endian" if v == "little" || v == "big" => t.endian = v.into(),
                        "arranque" => {
                            numero(v).ok_or_else(|| err("arranque invalido"))?;
                            t.arranque = v.into();
                        }
                        _ => return Err(err(&format!("opcion `{kv}` desconocida"))),
                    }
                }
            }
            "registros" => t
                .registros
                .extend(resto.split_whitespace().map(String::from)),
            "pc" | "sp" => {
                let rol = if clave.trim() == "pc" { "pc" } else { "sp" };
                let p: Vec<&str> = resto.split_whitespace().collect();
                if p.len() < 2 {
                    return Err(err("se esperaba `NOMBRE BITS [INICIAL]`"));
                }
                let bits = numero(p[1]).ok_or_else(|| err("bits invalidos"))? as u32;
                t.otros_registros
                    .push((p[0].into(), bits, rol, p.get(2).map(|s| s.to_string())));
            }
            "bandera" => {
                let mut p = resto.splitn(3, char::is_whitespace);
                let nombre = p.next().unwrap_or("").to_string();
                let bit = p
                    .next()
                    .and_then(numero)
                    .ok_or_else(|| err("falta el bit"))? as u32;
                t.banderas
                    .push((nombre, bit, p.next().unwrap_or("").trim().to_string()));
            }
            "region" => {
                let p: Vec<&str> = resto.split_whitespace().collect();
                if p.len() != 4 || numero(p[1]).is_none() || numero(p[2]).is_none() {
                    return Err(err("se esperaba `NOMBRE DESDE HASTA ACCESO`"));
                }
                t.regiones
                    .push((p[0].into(), p[1].into(), p[2].into(), p[3].into()));
            }
            "instruccion" => {
                let campos: Vec<&str> = resto.split(';').map(str::trim).collect();
                if campos.len() < 4 || campos.len() > 5 {
                    return Err(err(
                        "se esperaba `SINTAXIS ; CODIGO ; SEMANTICA ; CICLOS [; AFECTA]`",
                    ));
                }
                t.instrucciones.push(Instruccion {
                    sintaxis: campos[0].into(),
                    codigo: campos[1].into(),
                    semantica: campos[2].into(),
                    ciclos: campos[3].into(),
                    afecta: campos.get(4).map_or_else(Vec::new, |a| {
                        a.split_whitespace().map(String::from).collect()
                    }),
                    linea: n,
                });
            }
            otra => return Err(err(&format!("clave `{otra}` desconocida"))),
        }
    }
    if t.nombre.is_empty() {
        return Err("falta la linea `maquina:`".into());
    }
    if t.palabra == 0 || t.direccion == 0 {
        return Err("`maquina:` necesita palabra=N y direccion=N".into());
    }
    if t.instrucciones.is_empty() {
        return Err("no hay ninguna `instruccion:`".into());
    }
    Ok(t)
}

fn ancho_de_campo(codigo: &str, nombre: &str) -> Option<u32> {
    codigo
        .split(|c: char| c.is_whitespace() || c == '|')
        .find_map(|c| c.strip_prefix(nombre)?.strip_prefix(':')?.parse().ok())
}

fn operandos(i: &Instruccion, regs: &[String]) -> Result<String, String> {
    let mut s = String::new();
    let mut resto = i.sintaxis.as_str();
    while let Some(a) = resto.find('{') {
        let b = resto[a..]
            .find('}')
            .ok_or_else(|| format!("linea {}: falta `}}`", i.linea))?
            + a;
        let nombre = &resto[a + 1..b];
        resto = &resto[b + 1..];
        s.push_str("  [[instruccion.operando]]\n");
        s.push_str(&format!("  nombre = {}\n", comillas(nombre)));
        let base = nombre.trim_end_matches(|c: char| c.is_ascii_digit());
        let ancho = ancho_de_campo(&i.codigo, nombre);
        match base {
            "rd" | "rs" | "rt" | "ra" | "rb" | "r" | "reg" => {
                let lista = regs
                    .iter()
                    .map(|r| comillas(r))
                    .collect::<Vec<_>>()
                    .join(", ");
                s.push_str(&format!("  tipo = \"registro\"\n  de = [{lista}]\n"));
            }
            "imm" | "imms" => {
                let w = ancho.ok_or_else(|| {
                    format!(
                        "linea {}: el campo `{nombre}` no esta en el codigo",
                        i.linea
                    )
                })?;
                if base == "imms" {
                    s.push_str(&format!(
                        "  tipo = \"inmediato\"\n  signo = true\n  rango = [-{}, {}]\n",
                        1u64 << (w - 1),
                        (1u64 << (w - 1)) - 1
                    ));
                } else {
                    s.push_str(&format!(
                        "  tipo = \"inmediato\"\n  signo = false\n  rango = [0, {}]\n",
                        (1u64 << w) - 1
                    ));
                }
            }
            "dir" => s.push_str("  tipo = \"direccion\"\n"),
            "off" | "rel" => s.push_str("  tipo = \"desplazamiento\"\n"),
            _ => {
                return Err(format!(
                    "linea {}: no se que operando es `{nombre}`",
                    i.linea
                ))
            }
        }
    }
    Ok(s)
}

fn generar(t: &Tabla) -> Result<String, String> {
    let mut o = String::new();
    o.push_str(&format!(
        "# {} Revisa la semantica antes de usarla.\n\n[maquina]\nnombre = {}\nformato = \"0.1\"\nbits_palabra = {}\nbits_direccion = {}\nendianness = {}\narranque = {}\npila = \"descendente\"\n\n",
        lorenz_arch::marca_generado("lorenz importar"), comillas(&t.nombre), t.palabra, t.direccion, comillas(&t.endian), t.arranque
    ));
    if !t.banderas.is_empty() {
        o.push_str("[flags]\n");
        for (n, b, d) in &t.banderas {
            o.push_str(&format!(
                "{n} = {{ bit = {b}, descripcion = {} }}\n",
                comillas(d)
            ));
        }
        o.push('\n');
    }
    for r in &t.registros {
        o.push_str(&format!(
            "[[registro]]\nnombre = {}\nbits = {}\nrol = \"general\"\ninicial = 0\n\n",
            comillas(r),
            t.palabra
        ));
    }
    for (n, bits, rol, ini) in &t.otros_registros {
        o.push_str(&format!(
            "[[registro]]\nnombre = {}\nbits = {bits}\nrol = \"{rol}\"\n",
            comillas(n)
        ));
        if let Some(i) = ini {
            o.push_str(&format!("inicial = {i}\n"));
        }
        o.push('\n');
    }
    if !t.banderas.is_empty() {
        o.push_str("[[registro]]\nnombre = \"F\"\nbits = ");
        o.push_str(&format!("{}\nrol = \"flags\"\n\n", t.palabra));
    }
    for (n, d, h, a) in &t.regiones {
        o.push_str(&format!(
            "[[region]]\nnombre = {}\ndesde = {d}\nhasta = {h}\nacceso = {}\n\n",
            comillas(n),
            comillas(a)
        ));
    }
    for i in &t.instrucciones {
        let mnem = i.sintaxis.split_whitespace().next().unwrap_or("");
        o.push_str(&format!(
            "[[instruccion]]\nmnemonico = {}\nsintaxis = {}\ncodigo = {}\nsemantica = {}\n",
            comillas(mnem),
            comillas(&i.sintaxis),
            comillas(&i.codigo),
            comillas(&i.semantica)
        ));
        if !i.afecta.is_empty() {
            let l = i
                .afecta
                .iter()
                .map(|a| comillas(a))
                .collect::<Vec<_>>()
                .join(", ");
            o.push_str(&format!("afecta = [{l}]\n"));
        }
        match i.ciclos.split_once('/') {
            Some((a, b)) if numero(a).is_some() && numero(b).is_some() => {
                o.push_str(&format!(
                    "ciclos = {{ tomado = {}, no_tomado = {} }}\n",
                    a.trim(),
                    b.trim()
                ));
            }
            None if numero(&i.ciclos).is_some() => o.push_str(&format!("ciclos = {}\n", i.ciclos)),
            _ => {
                return Err(format!(
                    "linea {}: ciclos `{}` invalidos",
                    i.linea, i.ciclos
                ))
            }
        }
        o.push_str(&operandos(i, &t.registros)?);
        o.push('\n');
    }
    Ok(o)
}

/// Convierte una tabla en la descripcion TOML de la maquina, ya validada.
pub fn importar_texto(texto: &str) -> Result<String, String> {
    let t = leer(texto)?;
    let toml = generar(&t)?;
    lorenz_arch::cargar(&toml).map_err(|e| format!("la maquina generada no es valida: {e}"))?;
    Ok(toml)
}

pub fn importar_cli(
    archivo: &str,
    destino: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let texto = match std::fs::read_to_string(archivo) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(errores, "{archivo}: no se pudo leer: {e}");
            return SALIDA_ERROR_USO;
        }
    };
    let toml = match importar_texto(&texto) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(errores, "{archivo}: {e}");
            return SALIDA_ERROR_USO;
        }
    };
    match destino {
        Some(d) => {
            if let Err(e) = std::fs::write(d, &toml) {
                let _ = writeln!(errores, "{d}: no se pudo escribir: {e}");
                return SALIDA_ERROR_USO;
            }
            let _ = writeln!(salida, "{archivo}: importada en {d}");
        }
        None => {
            let _ = salida.write_all(toml.as_bytes());
        }
    }
    SALIDA_OK
}
