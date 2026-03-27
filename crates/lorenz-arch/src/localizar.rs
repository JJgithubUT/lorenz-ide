//! Situa en el archivo los fallos de validacion (RF-ARQ-034).
//!
//! La validacion trabaja sobre la maquina ya construida, que no recuerda en que linea estaba cada
//! cosa. Aqui se busca en el texto original la declaracion de lo que el fallo nombra, para que el
//! mensaje diga la linea donde corregirlo.

use crate::error::{FalloValidacion, Posicion, Regla};

/// Completa la posicion de los fallos que todavia no la tienen.
pub(crate) fn situar(texto: &str, fallos: &mut [FalloValidacion]) {
    let lineas: Vec<&str> = texto.lines().collect();
    for f in fallos.iter_mut().filter(|f| f.posicion.is_none()) {
        f.posicion = Some(buscar(&lineas, f));
    }
}

fn buscar(lineas: &[&str], f: &FalloValidacion) -> Posicion {
    if f.regla == Regla::ArranqueFueraDeRegion {
        if let Some(p) = clave(lineas, "arranque") {
            return p;
        }
    }
    let candidatas: Vec<Posicion> = nombres_citados(&f.detalle)
        .iter()
        .flat_map(|n| declaraciones(lineas, n))
        .collect();
    // Un nombre repetido se corrige en la segunda declaracion, no en la primera.
    let elegida = if f.regla == Regla::NombreDuplicado {
        candidatas.iter().max_by_key(|p| p.linea)
    } else {
        candidatas.first()
    };
    if let Some(p) = elegida {
        return *p;
    }
    cabecera(lineas, "[flags]")
        .filter(|_| f.detalle.contains("bandera"))
        .or_else(|| cabecera(lineas, "[maquina]"))
        .unwrap_or_else(|| Posicion::nueva(1, 1))
}

/// Los textos entre comillas simples del detalle, en el orden en que aparecen.
fn nombres_citados(detalle: &str) -> Vec<String> {
    detalle
        .split('\'')
        .skip(1)
        .step_by(2)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Lineas que declaran algo llamado `nombre`: `mnemonico = "X"`, `nombre = "X"` o la clave `X` de
/// la tabla de banderas.
fn declaraciones(lineas: &[&str], nombre: &str) -> Vec<Posicion> {
    let comillas = format!("\"{nombre}\"");
    lineas
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let t = l.trim_start();
            let sangria = l.len() - t.len();
            let declara = ["mnemonico", "nombre"].iter().any(|k| {
                t.strip_prefix(k)
                    .map(|r| r.trim_start().starts_with('=') && r.contains(&comillas))
                    .unwrap_or(false)
            });
            let bandera = t
                .strip_prefix(nombre)
                .map(|r| r.trim_start().starts_with('=') && r.contains("bit"))
                .unwrap_or(false);
            (declara || bandera).then(|| Posicion::nueva(i + 1, sangria + 1))
        })
        .collect()
}

fn clave(lineas: &[&str], nombre: &str) -> Option<Posicion> {
    lineas.iter().enumerate().find_map(|(i, l)| {
        let t = l.trim_start();
        t.strip_prefix(nombre)
            .filter(|r| r.trim_start().starts_with('='))
            .map(|_| Posicion::nueva(i + 1, l.len() - t.len() + 1))
    })
}

fn cabecera(lineas: &[&str], titulo: &str) -> Option<Posicion> {
    lineas
        .iter()
        .position(|l| l.trim() == titulo)
        .map(|i| Posicion::nueva(i + 1, 1))
}
