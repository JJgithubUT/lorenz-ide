//! Lo que el depurador muestra: texto para registros, memoria, codigo y paradas.

use std::fmt::Write;

use crate::depurador::{Depurador, Parada};
use lorenz_ens::desensamblador::{Desensamblador, Linea};

fn ancho_hex(d: &Depurador) -> usize {
    (d.maquina.bits_direccion as usize).div_ceil(4)
}

fn ancho_palabra(d: &Depurador) -> usize {
    (d.maquina.bits_palabra as usize).div_ceil(4)
}

/// Texto de la instruccion que empieza en `direccion`, si se puede desensamblar.
pub fn instruccion_en(d: &Depurador, direccion: u64) -> Option<String> {
    desensamblar(d, direccion, 1)
        .into_iter()
        .next()
        .map(|(_, t)| t)
}

/// Desensambla hasta `cantidad` instrucciones desde `desde`. Devuelve `(direccion, texto)`.
pub fn desensamblar(d: &Depurador, desde: u64, cantidad: usize) -> Vec<(u64, String)> {
    let Ok(des) = Desensamblador::nuevo(&d.maquina) else {
        return Vec::new();
    };
    let tope = d.maquina.direccion_maxima();
    let mut palabras = Vec::new();
    let mut a = desde;
    // Lo bastante para `cantidad` instrucciones largas.
    while palabras.len() < cantidad.saturating_mul(4).max(4) && a <= tope {
        match d.emu.memoria().leer_privilegiado(a) {
            Ok(p) => palabras.push(p),
            Err(_) => break,
        }
        a += 1;
    }
    des.desensamblar(desde, &palabras, Some(&d.etiquetas))
        .into_iter()
        .take(cantidad)
        .map(|l| match l {
            Linea::Instruccion {
                direccion, texto, ..
            } => (direccion, texto),
            Linea::Dato { direccion, palabra } => (direccion, format!(".dw {palabra:#x}")),
        })
        .collect()
}

/// `registros` y banderas, en el orden de la maquina (RF-DEP-005).
pub fn registros(d: &Depurador) -> String {
    let e = d.emu.estado();
    let w = ancho_palabra(d);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} = {:0aw$x}",
        e.nombre_del_pc(),
        e.pc(),
        aw = ancho_hex(d)
    );
    if let (Some(n), Some(v)) = (e.nombre_del_sp(), e.sp()) {
        let _ = writeln!(s, "{n} = {v:0aw$x}", aw = ancho_hex(d));
    }
    for n in e.nombres() {
        if let Ok(v) = e.registro(n) {
            let _ = writeln!(s, "{n} = {v:0w$x}  ({v})");
        }
    }
    let banderas: Vec<String> = e
        .nombres_de_banderas()
        .into_iter()
        .map(|b| format!("{b}={}", u8::from(e.bandera(b).unwrap_or(false))))
        .collect();
    if !banderas.is_empty() {
        let _ = writeln!(s, "banderas: {}", banderas.join(" "));
    }
    let _ = writeln!(
        s,
        "instrucciones: {}  ciclos: {}",
        e.instrucciones(),
        e.ciclos()
    );
    s
}

/// Volcado hexadecimal con ASCII (RF-DEP-006).
pub fn volcar(d: &Depurador, desde: u64, cantidad: usize) -> Result<String, crate::ErrorDepurador> {
    let palabras = d.emu.memoria().volcar(desde, cantidad)?;
    let w = ancho_palabra(d);
    let mut s = String::new();
    for (i, trozo) in palabras.chunks(8).enumerate() {
        let a = desde + (i * 8) as u64;
        let hex: Vec<String> = trozo.iter().map(|p| format!("{p:0w$x}")).collect();
        let ascii: String = trozo
            .iter()
            .map(|&p| {
                if (0x20..0x7f).contains(&p) {
                    p as u8 as char
                } else {
                    '.'
                }
            })
            .collect();
        let _ = writeln!(
            s,
            "{a:0aw$x}: {:<hw$} |{ascii}|",
            hex.join(" "),
            aw = ancho_hex(d),
            hw = 8 * (w + 1) - 1
        );
    }
    Ok(s)
}

/// Codigo alrededor de `desde`, con la linea de fuente cuando se conoce (RF-DEP-008).
pub fn listar(d: &Depurador, desde: u64, cantidad: usize) -> String {
    let pc = d.emu.estado().pc();
    let mut s = String::new();
    for (dir, texto) in desensamblar(d, desde, cantidad) {
        let marca = if dir == pc { "=>" } else { "  " };
        if let Some(n) = d.etiqueta_en(dir) {
            let _ = writeln!(s, "        {n}:");
        }
        let origen = d
            .ubicacion_de(dir)
            .and_then(|(a, l)| {
                d.fuentes
                    .get(a)
                    .and_then(|v| v.get(l.saturating_sub(1)))
                    .map(|t| format!("   ; {a}:{l}: {}", t.trim()))
            })
            .unwrap_or_default();
        let _ = writeln!(s, "{marca} {dir:0aw$x}  {texto}{origen}", aw = ancho_hex(d));
    }
    s
}

/// El contenido de la pila, de la cima hacia abajo (RF-DEP-009).
pub fn pila(d: &Depurador, cantidad: usize) -> Result<String, crate::ErrorDepurador> {
    use lorenz_arch::modelo::SentidoPila;
    let sp = d.emu.estado().sp().ok_or(crate::ErrorDepurador::SinPila)?;
    let w = ancho_palabra(d);
    let mut s = String::new();
    for i in 0..cantidad as u64 {
        let a = match d.maquina.pila {
            SentidoPila::Descendente => sp.wrapping_add(i),
            SentidoPila::Ascendente => sp.wrapping_sub(i + 1),
        };
        match d.emu.memoria().leer_privilegiado(a) {
            Ok(v) => {
                let _ = writeln!(s, "{a:0aw$x}: {v:0w$x}", aw = ancho_hex(d));
            }
            Err(_) => break,
        }
    }
    Ok(s)
}

/// Por que se detuvo la ejecucion, y donde (RF-DEP-016).
pub fn parada(d: &Depurador, p: &Parada) -> String {
    let e = d.emu.estado();
    let donde = format!("pc={:0aw$x}", e.pc(), aw = ancho_hex(d));
    match p {
        Parada::Paso { instrucciones } => {
            format!("{instrucciones} instruccion(es) ejecutada(s); {donde}")
        }
        Parada::Detenida => format!("la maquina se detuvo; {donde}"),
        Parada::Ruptura { id, direccion } => {
            format!("ruptura {id} en {direccion:#x}; {donde}")
        }
        Parada::Observacion { id, acceso, valor } => format!(
            "observacion {id}: {} de {:#x} por la instruccion en {:#x}{}",
            match acceso.tipo {
                lorenz_emu::TipoAcceso::Lectura => "lectura",
                lorenz_emu::TipoAcceso::Escritura => "escritura",
            },
            acceso.direccion,
            acceso.pc,
            valor
                .map(|v| format!("; valor ahora {v:#x}"))
                .unwrap_or_default()
        ),
        Parada::Llegada { direccion } => format!("se llego a {direccion:#x}"),
        Parada::Retorno => format!("se volvio de la subrutina; {donde}"),
        Parada::Limite { instrucciones } => {
            format!("limite alcanzado tras {instrucciones} instrucciones; {donde}")
        }
        Parada::Fallo {
            error,
            direccion,
            instruccion,
        } => format!(
            "fallo en {direccion:#x}{}: {error}",
            instruccion
                .as_ref()
                .map(|i| format!(" (`{i}`)"))
                .unwrap_or_default()
        ),
        Parada::CondicionInvalida { id, motivo } => {
            format!("la condicion de la ruptura {id} no se pudo evaluar: {motivo}")
        }
    }
}
