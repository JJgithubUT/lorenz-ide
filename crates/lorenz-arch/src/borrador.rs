//! Borrador de arquitectura a partir de una descripcion en lenguaje natural (RF-ECO-045).
//!
//! No usa ningun modelo: lee la descripcion con reglas fijas (numeros, palabras clave) y arma una
//! maquina con un formato de instruccion regular. Siempre es determinista, y es un punto de
//! partida: el resultado se edita como cualquier otra descripcion. Cada decision que no venia en
//! el texto se devuelve en `supuestos`, para que quien lo lea sepa que revisar. El borrador solo
//! se entrega si pasa la validacion del cargador.

use std::fmt::Write;

/// El resultado: el TOML y lo que se decidio sin que el texto lo dijera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Borrador {
    pub texto: String,
    pub supuestos: Vec<String>,
}

fn sin_acentos(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

fn numero_de_palabra(t: &str) -> Option<u64> {
    if let Ok(n) = t.parse() {
        return Some(n);
    }
    Some(match t {
        "dos" => 2,
        "tres" => 3,
        "cuatro" => 4,
        "seis" => 6,
        "ocho" => 8,
        "dieciseis" => 16,
        "treinta" => 30,
        "veinticuatro" => 24,
        _ => return None,
    })
}

fn multiplicador(unidad: &str) -> Option<u64> {
    match unidad {
        "palabras" | "palabra" | "bytes" | "byte" | "celdas" | "celda" => Some(1),
        "k" | "kb" | "kib" | "kilobytes" | "kilobyte" | "kilopalabras" => Some(1024),
        "m" | "mb" | "mib" | "megabytes" | "megabyte" => Some(1024 * 1024),
        _ => None,
    }
}

fn log2_techo(n: u64) -> u32 {
    if n <= 1 {
        0
    } else {
        64 - (n - 1).leading_zeros()
    }
}

#[derive(Default)]
struct Lectura {
    bits_palabra: Option<u32>,
    bits_direccion: Option<u32>,
    registros: Option<u64>,
    memoria: Option<u64>,
    nombres: Vec<String>,
    big: Option<bool>,
    nombre: Option<String>,
    aritmeticas: Vec<&'static str>,
    saltos: bool,
    pantalla: bool,
}

fn leer(descripcion: &str) -> Lectura {
    let limpio = sin_acentos(&descripcion.to_lowercase());
    let t: Vec<&str> = limpio
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut l = Lectura::default();
    let ventana = |i: usize, antes: usize, claves: &[&str]| {
        let desde = i.saturating_sub(antes);
        t[desde..i]
            .iter()
            .any(|x| claves.iter().any(|c| x.starts_with(c)))
    };
    for i in 0..t.len() {
        let Some(n) = numero_de_palabra(t[i]) else {
            continue;
        };
        let Some(sig) = t.get(i + 1) else { continue };
        if *sig == "bits" || *sig == "bit" {
            if ventana(i, 4, &["direccion"]) {
                l.bits_direccion.get_or_insert(n as u32);
            } else if !ventana(i, 3, &["registro"]) {
                l.bits_palabra.get_or_insert(n as u32);
            }
        } else if sig.starts_with("registro") {
            l.registros.get_or_insert(n);
        } else if let Some(m) = multiplicador(sig) {
            let cerca = ventana(i, 4, &["memoria", "ram"])
                || t[i + 1..(i + 5).min(t.len())]
                    .iter()
                    .any(|x| x.starts_with("memoria") || *x == "ram");
            if cerca {
                l.memoria.get_or_insert(n * m);
            }
        }
    }
    // Nombres de registros: «registros A, B, C y D» (hasta el primer trozo que no sea un nombre).
    if let Some(p) = limpio.find("registros ") {
        let resto = limpio[p + "registros ".len()..].replace(" y ", ",");
        for trozo in resto.split(',') {
            let x = trozo.trim().trim_end_matches('.');
            let nombre = !x.is_empty()
                && x.len() <= 3
                && x.chars().next().is_some_and(|c| c.is_alphabetic())
                && x.chars().all(|c| c.is_alphanumeric())
                && !matches!(x, "de" | "con" | "en");
            if nombre {
                l.nombres.push(x.to_uppercase());
            } else {
                break;
            }
        }
    }
    l.big = if t.iter().any(|x| *x == "big" || x.starts_with("gran")) {
        Some(true)
    } else if t.iter().any(|x| *x == "little" || x.starts_with("peque")) {
        Some(false)
    } else {
        None
    };
    if let Some(p) = t
        .iter()
        .position(|x| *x == "llamada" || *x == "llamado" || *x == "nombre")
    {
        if let Some(n) = t.get(p + 1) {
            if n.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                l.nombre = Some((*n).to_string());
            }
        }
    }
    let tiene = |cs: &[&str]| t.iter().any(|x| cs.iter().any(|c| x.starts_with(c)));
    for (clave, mn) in [
        (&["sum", "add"][..], "ADD"),
        (&["rest", "sub"][..], "SUB"),
        (&["xor", "exclusiv"][..], "XOR"),
    ] {
        if tiene(clave) {
            l.aritmeticas.push(mn);
        }
    }
    if tiene(&["logic", "and"]) || t.contains(&"y") && tiene(&["logic"]) {
        l.aritmeticas.push("AND");
        l.aritmeticas.push("OR");
    }
    l.saltos = tiene(&["salto", "bucle", "condicion", "bifurc", "jump"]);
    l.pantalla = tiene(&["pantalla", "video", "display"]);
    l
}

/// Una instruccion del borrador, ya con su codigo.
struct Instr {
    mn: &'static str,
    forma: Forma,
    semantica: &'static str,
    ciclos: &'static str,
    afecta: bool,
    texto: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
enum Forma {
    Nada,
    RegReg,
    RegImm,
    RegDir,
    DirReg,
    Dir,
}

fn catalogo(mn: &str) -> Instr {
    use Forma::*;
    let (forma, semantica, ciclos, afecta, texto) = match mn {
        "MOV" => (RegReg, "rd = rs", "1", false, "Copia un registro en otro."),
        "LDI" => (
            RegImm,
            "rd = imm",
            "2",
            false,
            "Carga una constante en un registro.",
        ),
        "ADD" => (RegReg, "rd = rd + rs", "2", true, "Suma dos registros."),
        "SUB" => (
            RegReg,
            "rd = rd - rs",
            "2",
            true,
            "Resta el segundo registro del primero.",
        ),
        "AND" => (RegReg, "rd = rd & rs", "2", true, "Y logico bit a bit."),
        "OR" => (RegReg, "rd = rd | rs", "2", true, "O logico bit a bit."),
        "XOR" => (RegReg, "rd = rd ^ rs", "2", true, "O exclusivo bit a bit."),
        "LD" => (
            RegDir,
            "rd = mem[dir]",
            "4",
            false,
            "Lee memoria en un registro.",
        ),
        "ST" => (
            DirReg,
            "mem[dir] = rs",
            "4",
            false,
            "Escribe un registro en memoria.",
        ),
        "JMP" => (Dir, "pc = dir", "3", false, "Salta a una direccion."),
        "JZ" => (
            Dir,
            "si Z: pc = dir",
            "{ tomado = 3, no_tomado = 2 }",
            false,
            "Salta si Z esta encendida.",
        ),
        "JNZ" => (
            Dir,
            "si !Z: pc = dir",
            "{ tomado = 3, no_tomado = 2 }",
            false,
            "Salta si Z esta apagada.",
        ),
        "JC" => (
            Dir,
            "si C: pc = dir",
            "{ tomado = 3, no_tomado = 2 }",
            false,
            "Salta si hay acarreo.",
        ),
        "NOP" => (Nada, "", "1", false, "No hace nada."),
        _ => (Nada, "detener", "1", false, "Detiene la maquina."),
    };
    Instr {
        mn: Box::leak(mn.to_string().into_boxed_str()),
        forma,
        semantica,
        ciclos,
        afecta,
        texto,
    }
}

fn hex(n: u64) -> String {
    format!("0x{n:X}")
}

/// Arma el borrador. Falla si la descripcion pide algo que este formato no puede expresar.
pub fn desde_descripcion(descripcion: &str) -> Result<Borrador, String> {
    if descripcion.trim().is_empty() {
        return Err("la descripcion esta vacia".into());
    }
    let l = leer(descripcion);
    let mut sup: Vec<String> = Vec::new();

    let w = match l.bits_palabra {
        Some(b) if (8..=32).contains(&b) && b % 8 == 0 => b,
        Some(b) => {
            return Err(format!(
                "palabras de {b} bits: este borrador admite 8, 16, 24 o 32 (multiplos de 8)"
            ))
        }
        None => {
            sup.push("Palabras de 8 bits (no se indico).".into());
            8
        }
    };
    let mut a = match (l.bits_direccion, l.memoria) {
        (Some(a), _) => a,
        (None, Some(m)) => {
            let a = log2_techo(m).max(4);
            sup.push(format!(
                "Direcciones de {a} bits, las que pide una memoria de {m} palabras."
            ));
            a
        }
        (None, None) => {
            let a = if w == 8 { 16 } else { w.min(16) };
            sup.push(format!("Direcciones de {a} bits (no se indico)."));
            a
        }
    };
    if a > 16 {
        sup.push(format!(
            "Direcciones limitadas a 16 bits (se pidieron {a})."
        ));
        a = 16;
    }
    if a < 4 {
        return Err("las direcciones necesitan al menos 4 bits".into());
    }
    // Una direccion cabe en una palabra, o en un numero exacto de palabras.
    let piezas_dir = a.div_ceil(w) as usize;
    let k_max = (w - 4) / 2;
    let pedidas = if !l.nombres.is_empty() {
        l.nombres.len() as u64
    } else {
        l.registros.unwrap_or(4)
    };
    if l.registros.is_none() && l.nombres.is_empty() {
        sup.push("4 registros de uso general (no se indico).".into());
    }
    if pedidas < 2 {
        return Err("hacen falta al menos 2 registros".into());
    }
    let mut k = log2_techo(pedidas).max(1);
    if k > k_max.min(4) {
        k = k_max.min(4);
        sup.push(format!(
            "Se piden {pedidas} registros pero con palabras de {w} bits caben {}; se usan esos.",
            1u32 << k
        ));
    }
    let n = 1usize << k;
    let nombres: Vec<String> = if l.nombres.len() == n {
        l.nombres.clone()
    } else {
        if !l.nombres.is_empty() {
            sup.push("Los nombres de registro no coinciden con la cantidad: se usan los predeterminados.".into());
        }
        if n <= 4 {
            ["A", "B", "C", "D"][..n]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            (0..n).map(|i| format!("R{i}")).collect()
        }
    };
    if n as u64 != pedidas && l.nombres.is_empty() && pedidas <= (1 << k_max.min(4)) {
        sup.push(format!(
            "{pedidas} registros se redondean a {n} (potencia de dos)."
        ));
    }
    let big = l.big.unwrap_or(false);
    if l.big.is_none() && piezas_dir > 1 {
        sup.push("Orden de bytes little endian (no se indico).".into());
    }

    // Instrucciones: las pedidas mas la base.
    let mut ops: Vec<&'static str> = vec!["MOV", "LDI"];
    if l.aritmeticas.is_empty() {
        ops.extend(["ADD", "SUB"]);
        sup.push("Instrucciones aritmeticas ADD y SUB (no se indicaron).".into());
    } else {
        for o in &l.aritmeticas {
            if !ops.contains(o) {
                ops.push(o);
            }
        }
    }
    ops.extend(["LD", "ST", "JMP"]);
    ops.extend(["JZ", "JNZ"]);
    if l.saltos {
        ops.push("JC");
    }
    if ops.len() > 14 {
        ops.truncate(14);
        sup.push("Se omitieron instrucciones: el formato de 4 bits de operacion admite 16.".into());
    }
    let con_flags = true;

    let mut t = String::new();
    let nombre = l.nombre.unwrap_or_else(|| format!("borrador-{w}"));
    let _ = writeln!(
        t,
        "# Borrador generado por Lorenz {} con `lorenz borrador`.",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(
        t,
        "# Borrador a partir de: {}",
        descripcion.trim().replace('\n', " ")
    );
    let _ = writeln!(
        t,
        "# Revisa y edita lo que haga falta: es un punto de partida.\n"
    );
    let _ = writeln!(
        t,
        "[maquina]\nnombre         = \"{nombre}\"\nformato        = \"0.1\""
    );
    let _ = writeln!(t, "bits_palabra   = {w}\nbits_direccion = {a}");
    let _ = writeln!(
        t,
        "endianness     = \"{}\"\narranque       = 0x0000\npila           = \"descendente\"\n",
        if big { "big" } else { "little" }
    );
    if con_flags {
        let _ = writeln!(
            t,
            "[flags]\nZ = {{ bit = 0, descripcion = \"resultado igual a cero\" }}"
        );
        let _ = writeln!(t, "C = {{ bit = 1, descripcion = \"acarreo\" }}\n");
    }
    for r in &nombres {
        let _ = writeln!(
            t,
            "[[registro]]\nnombre = \"{r}\"\nbits = {w}\nrol = \"general\"\ninicial = 0\n"
        );
    }
    let _ = writeln!(
        t,
        "[[registro]]\nnombre = \"PC\"\nbits = {a}\nrol = \"pc\"\n"
    );
    if con_flags {
        let _ = writeln!(
            t,
            "[[registro]]\nnombre = \"FL\"\nbits = {w}\nrol = \"flags\"\n"
        );
    }

    let lista = nombres
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let dir_campos = if a <= w {
        format!("dir:{a}")
    } else {
        vec![format!("dir:{w}"); piezas_dir].join(" | ")
    };
    let relleno = |bits: u32| {
        if bits == 0 {
            String::new()
        } else {
            ".".repeat(bits as usize)
        }
    };
    let mut todas: Vec<Instr> = vec![catalogo("NOP")];
    todas.extend(ops.iter().map(|m| catalogo(m)));
    todas.push(catalogo("HLT"));
    for (i, ins) in todas.iter().enumerate() {
        let codigo_op = if ins.mn == "HLT" { 15 } else { i as u64 };
        let op = format!("{:04b}", codigo_op);
        let codigo = match ins.forma {
            Forma::Nada => format!("{op} {}", relleno(w - 4)),
            Forma::RegReg => format!("{op} rd:{k} rs:{k} {}", relleno(w - 4 - 2 * k)),
            Forma::RegImm => format!("{op} rd:{k} {} | imm:{w}", relleno(w - 4 - k)),
            Forma::RegDir => format!("{op} rd:{k} {} | {dir_campos}", relleno(w - 4 - k)),
            Forma::DirReg => format!("{op} rs:{k} {} | {dir_campos}", relleno(w - 4 - k)),
            Forma::Dir => format!("{op} {} | {dir_campos}", relleno(w - 4)),
        };
        let codigo = codigo.replace("  ", " ").replace(" |  ", " | ");
        let sintaxis = match ins.forma {
            Forma::Nada => ins.mn.to_string(),
            Forma::RegReg => format!("{} {{rd}}, {{rs}}", ins.mn),
            Forma::RegImm => format!("{} {{rd}}, {{imm}}", ins.mn),
            Forma::RegDir => format!("{} {{rd}}, [{{dir}}]", ins.mn),
            Forma::DirReg => format!("{} [{{dir}}], {{rs}}", ins.mn),
            Forma::Dir => format!("{} {{dir}}", ins.mn),
        };
        let _ = writeln!(
            t,
            "[[instruccion]]\nmnemonico = \"{}\"\nsintaxis  = \"{sintaxis}\"",
            ins.mn
        );
        let _ = writeln!(
            t,
            "codigo    = \"{}\"\nsemantica = \"{}\"",
            codigo.trim(),
            ins.semantica
        );
        if ins.afecta {
            let _ = writeln!(t, "afecta    = [\"Z\", \"C\"]");
        }
        let _ = writeln!(
            t,
            "ciclos    = {}\ndescripcion = \"{}\"",
            ins.ciclos, ins.texto
        );
        let operando = |nom: &str| {
            match nom {
            "rd" | "rs" => format!(
                "  [[instruccion.operando]]\n  nombre = \"{nom}\"\n  tipo = \"registro\"\n  de = [{lista}]\n"
            ),
            "imm" => format!(
                "  [[instruccion.operando]]\n  nombre = \"imm\"\n  tipo = \"inmediato\"\n  signo = false\n  rango = [0, {}]\n",
                (1u64 << w) - 1
            ),
            _ => "  [[instruccion.operando]]\n  nombre = \"dir\"\n  tipo = \"direccion\"\n".to_string(),
        }
        };
        match ins.forma {
            Forma::RegReg => {
                let _ = write!(t, "{}{}", operando("rd"), operando("rs"));
            }
            Forma::RegImm => {
                let _ = write!(t, "{}{}", operando("rd"), operando("imm"));
            }
            Forma::RegDir => {
                let _ = write!(t, "{}{}", operando("rd"), operando("dir"));
            }
            Forma::DirReg => {
                let _ = write!(t, "{}{}", operando("rs"), operando("dir"));
            }
            Forma::Dir => {
                let _ = write!(t, "{}", operando("dir"));
            }
            Forma::Nada => {}
        }
        let _ = writeln!(t);
    }

    // Memoria: un octavo de ROM y el resto RAM; la pantalla, si se pidio, al final.
    let tope: u64 = (1u64 << a) - 1;
    let rom_fin = (1u64 << a) / 8 - 1;
    let pantalla = l.pantalla && w == 8 && a >= 12;
    if l.pantalla && !pantalla {
        sup.push(
            "No se agrego pantalla: este borrador solo la coloca con palabras de 8 bits.".into(),
        );
    }
    let ram_fin = if pantalla { tope - 256 } else { tope };
    let _ = writeln!(
        t,
        "[[region]]\nnombre = \"rom\"\ndesde  = {}\nhasta  = {}\nacceso = \"r\"\n",
        hex(0),
        hex(rom_fin)
    );
    let _ = writeln!(
        t,
        "[[region]]\nnombre = \"ram\"\ndesde  = {}\nhasta  = {}\nacceso = \"rw\"\n",
        hex(rom_fin + 1),
        hex(ram_fin)
    );
    sup.push(format!(
        "Memoria: ROM en 0x0..{} y RAM en {}..{}; el programa arranca en 0x0.",
        hex(rom_fin),
        hex(rom_fin + 1),
        hex(ram_fin)
    ));
    if pantalla {
        let _ = writeln!(
            t,
            "[[region]]\nnombre      = \"video\"\ndesde       = {}\nhasta       = {}\nacceso      = \"rw\"\ndispositivo = \"pantalla\"\n",
            hex(ram_fin + 1),
            hex(tope)
        );
        let _ = writeln!(t, "[[dispositivo]]\nnombre  = \"pantalla\"\ntipo    = \"pantalla\"\nancho   = 64\nalto    = 32\nformato = \"indice1\"");
    }
    sup.push("Banderas Z (cero) y C (acarreo) en el registro FL.".into());

    crate::cargar(&t).map_err(|e| format!("el borrador no paso la validacion: {e}"))?;
    Ok(Borrador {
        texto: t,
        supuestos: sup,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ok(d: &str) -> Borrador {
        desde_descripcion(d).unwrap_or_else(|e| panic!("{d}: {e}"))
    }

    #[test]
    fn una_descripcion_minima_da_una_maquina_valida() {
        let b = ok("Una maquina de 8 bits con 4 registros");
        let m = crate::cargar(&b.texto).unwrap();
        assert_eq!(m.bits_palabra, 8);
        assert_eq!(
            m.registros.iter().filter(|r| r.nombre.len() == 1).count(),
            4
        );
        assert!(b.supuestos.iter().any(|s| s.contains("no se indic")));
    }

    #[test]
    fn lee_bits_registros_memoria_y_orden() {
        let b =
            ok("Procesador de 16 bits, 8 registros, memoria de 64 KiB, big endian, llamada mini");
        let m = crate::cargar(&b.texto).unwrap();
        assert_eq!((m.bits_palabra, m.bits_direccion), (16, 16));
        assert!(b.texto.contains("nombre         = \"mini\""));
        assert!(b.texto.contains("endianness     = \"big\""));
        assert_eq!(
            m.registros
                .iter()
                .filter(|r| r.nombre.starts_with('R') && r.nombre != "R")
                .count(),
            8
        );
    }

    #[test]
    fn los_nombres_de_registro_se_respetan() {
        let b = ok("8 bits con registros X, Y, Z y W");
        assert!(
            b.texto.contains("nombre = \"X\"") && b.texto.contains("nombre = \"W\""),
            "{}",
            b.texto
        );
    }

    #[test]
    fn pide_pantalla_y_operaciones() {
        let b = ok("Maquina de 8 bits con pantalla, suma, resta y operaciones logicas");
        for m in ["ADD", "SUB", "AND", "OR"] {
            assert!(b.texto.contains(&format!("mnemonico = \"{m}\"")), "{m}");
        }
        assert!(b.texto.contains("dispositivo = \"pantalla\""));
    }

    #[test]
    fn es_determinista_y_lleva_la_marca() {
        let d = "8 bits, 4 registros";
        assert_eq!(ok(d), ok(d));
        assert!(ok(d).texto.contains("`lorenz borrador`"));
    }

    #[test]
    fn lo_imposible_se_explica() {
        assert!(desde_descripcion("").is_err());
        assert!(desde_descripcion("palabras de 12 bits")
            .unwrap_err()
            .contains("12"));
        assert!(desde_descripcion("8 bits con 1 registros")
            .unwrap_err()
            .contains("2 registros"));
    }

    #[test]
    fn muchos_registros_en_8_bits_se_reducen_y_se_avisa() {
        let b = ok("8 bits con 8 registros");
        assert!(b.supuestos.iter().any(|s| s.contains("caben")));
    }

    #[test]
    fn el_borrador_ensambla_y_ejecuta_en_el_cargador() {
        // Cada combinacion habitual debe ser valida.
        for d in [
            "8 bits",
            "16 bits, 8 registros, 4K palabras de memoria",
            "16 bits con 4 registros y saltos condicionales",
            "8 bits, direcciones de 8 bits",
        ] {
            ok(d);
        }
    }
}
