//! Las ordenes `ensamblar` y `desensamblar`, y la lectura de fuentes para `ejecutar` (RF-ENS-026).

use std::io::Write;

use lorenz_arch::modelo::Maquina;
use lorenz_ens::desensamblador::Desensamblador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente, Objeto};
use lorenz_ens::preprocesar::CargadorDeDisco;
use lorenz_ens::salidas::{leer_etiquetas, listado, mapa, tabla_de_simbolos};

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};
use crate::programa::{escribir_palabras, leer_palabras};

/// Extensiones que se tratan como codigo fuente en ensamblador.
pub const EXTENSIONES_FUENTE: [&str; 3] = ["s", "asm", "lasm"];

/// `true` si la ruta parece un fuente en ensamblador y no un programa en hexadecimal.
pub fn es_fuente(ruta: &str) -> bool {
    std::path::Path::new(ruta)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONES_FUENTE.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Lee y ensambla un archivo, siguiendo sus `.incluir`. Devuelve tambien todos los archivos
/// leidos (el principal primero). Los errores salen ya formateados, uno por linea.
pub fn ensamblar_archivo(maquina: &Maquina, ruta: &str) -> Result<(Objeto, Vec<Fuente>), String> {
    let texto =
        std::fs::read_to_string(ruta).map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
    let raiz = Fuente::nueva(ruta, texto);
    let ensamblador = Ensamblador::nuevo(maquina).map_err(|e| format!("{ruta}: {e}"))?;
    ensamblador
        .ensamblar_con(&raiz, &CargadorDeDisco)
        .map_err(|errores| {
            errores
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })
}

/// Como [`ensamblar_archivo`], pero para programas que se van a ejecutar: no admite simbolos
/// externos sin enlazar, porque sus instrucciones estarian vacias.
pub fn ensamblar_ejecutable(
    maquina: &Maquina,
    ruta: &str,
) -> Result<(Objeto, Vec<Fuente>), String> {
    let (objeto, fuentes) = ensamblar_archivo(maquina, ruta)?;
    if objeto.reubicaciones.is_empty() {
        Ok((objeto, fuentes))
    } else {
        Err(format!(
            "{ruta}: usa simbolos externos ({}); enlazalo con `lorenz enlazar` antes de ejecutarlo",
            objeto
                .externos
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

/// Opciones de `lorenz ensamblar`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpcionesEnsamblar {
    pub salida: Option<String>,
    pub mapa: Option<String>,
    pub listado: Option<String>,
    pub simbolos: Option<String>,
    pub objeto: Option<String>,
}

fn escribir_archivo(ruta: &str, texto: &str, errores: &mut dyn Write) -> bool {
    match std::fs::write(ruta, texto) {
        Ok(()) => true,
        Err(e) => {
            escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
            false
        }
    }
}

/// Imagen continua desde la direccion mas baja hasta la mas alta, con ceros en los huecos.
pub fn imagen_hex(objeto: &Objeto, maquina: &Maquina) -> String {
    let palabras = objeto.palabras();
    let Some(&(origen, _)) = palabras.first() else {
        return "# programa vacio\n".to_string();
    };
    let ultimo = palabras.last().map_or(origen, |p| p.0);
    let mut imagen = vec![0u64; (ultimo - origen + 1) as usize];
    for (d, p) in &palabras {
        imagen[(d - origen) as usize] = *p;
    }
    let digitos = (maquina.bits_direccion as usize).div_ceil(4);
    let mut texto = format!("# origen {origen:#0ancho$x}\n", ancho = digitos + 2);
    if imagen.len() != palabras.len() {
        texto.push_str("# aviso: los huecos entre segmentos se rellenan con ceros\n");
    }
    texto.push_str(&escribir_palabras(&imagen, maquina.bits_palabra, 16));
    texto
}

pub fn ensamblar_cli(
    ruta_maquina: &str,
    ruta_fuente: &str,
    opciones: &OpcionesEnsamblar,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let (objeto, fuentes) = match ensamblar_archivo(&maquina, ruta_fuente) {
        Ok(r) => r,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    for a in &objeto.avisos {
        escribir(errores, &format!("{a}\n"));
    }
    if !objeto.reubicaciones.is_empty() {
        // El programa depende de simbolos de otros archivos: solo el objeto sirve para enlazar.
        let Some(ruta) = &opciones.objeto else {
            escribir(
                errores,
                &format!(
                    "{ruta_fuente}: usa simbolos externos ({}); guardalo con --objeto y enlazalo con `lorenz enlazar`\n",
                    objeto.externos.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            );
            return SALIDA_ERROR_USO;
        };
        if !escribir_archivo(ruta, &objeto.a_toml(&maquina.nombre), errores) {
            return SALIDA_ERROR_USO;
        }
        escribir(
            salida,
            &format!(
                "{ruta_fuente}: objeto con {} instruccion(es) por enlazar\n",
                objeto.reubicaciones.len()
            ),
        );
        return SALIDA_OK;
    }
    let hex = imagen_hex(&objeto, &maquina);
    match &opciones.salida {
        Some(ruta) => {
            if !escribir_archivo(ruta, &hex, errores) {
                return SALIDA_ERROR_USO;
            }
        }
        None => escribir(salida, &hex),
    }
    if let Some(ruta) = &opciones.mapa {
        if !escribir_archivo(ruta, &mapa(&objeto), errores) {
            return SALIDA_ERROR_USO;
        }
    }
    if let Some(ruta) = &opciones.listado {
        let texto = listado(&objeto, &fuentes, maquina.bits_palabra);
        if !escribir_archivo(ruta, &texto, errores) {
            return SALIDA_ERROR_USO;
        }
    }
    if let Some(ruta) = &opciones.simbolos {
        if !escribir_archivo(ruta, &tabla_de_simbolos(&objeto), errores) {
            return SALIDA_ERROR_USO;
        }
    }
    if let Some(ruta) = &opciones.objeto {
        if !escribir_archivo(ruta, &objeto.a_toml(&maquina.nombre), errores) {
            return SALIDA_ERROR_USO;
        }
    }
    if opciones.salida.is_some() {
        escribir(
            salida,
            &format!(
                "{ruta_fuente}: {} palabras en {} segmento(s)\n",
                objeto.tamano(),
                objeto.segmentos.len()
            ),
        );
    }
    SALIDA_OK
}

pub fn desensamblar_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    direccion: Option<u64>,
    ruta_simbolos: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let texto = match std::fs::read_to_string(ruta_programa) {
        Ok(t) => t,
        Err(e) => {
            escribir(errores, &format!("{ruta_programa}: no se pudo leer: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let palabras = match leer_palabras(&texto, maquina.bits_palabra) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let simbolos = match ruta_simbolos {
        None => None,
        Some(r) => match std::fs::read_to_string(r)
            .map_err(|e| format!("{r}: no se pudo leer: {e}"))
            .and_then(|t| leer_etiquetas(&t).map_err(|e| format!("{r}: {e}")))
        {
            Ok(s) => Some(s),
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        },
    };
    let desensamblador = match Desensamblador::nuevo(&maquina) {
        Ok(d) => d,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let inicio = direccion.unwrap_or(maquina.arranque);
    let lineas = desensamblador.desensamblar(inicio, &palabras, simbolos.as_ref());
    escribir(salida, &desensamblador.a_texto(&lineas, simbolos.as_ref()));
    SALIDA_OK
}

/// Lee un programa para ejecutarlo y devuelve los tramos a cargar como `(direccion, palabras)`.
///
/// Un fuente en ensamblador (por extension) se ensambla y cada segmento va a su direccion; un
/// programa en hexadecimal es un solo tramo que empieza en `inicio`.
pub fn cargar_segmentos(
    maquina: &Maquina,
    ruta: &str,
    inicio: u64,
) -> Result<Vec<(u64, Vec<u64>)>, String> {
    if crate::compilar::es_sgt(ruta) {
        let (c, _) = crate::compilar::compilar_archivo(maquina, ruta, None)?;
        return Ok(c
            .objeto
            .segmentos
            .into_iter()
            .map(|s| (s.direccion, s.palabras))
            .collect());
    }
    if es_fuente(ruta) {
        let (objeto, _) = ensamblar_ejecutable(maquina, ruta)?;
        return Ok(objeto
            .segmentos
            .into_iter()
            .map(|s| (s.direccion, s.palabras))
            .collect());
    }
    let texto =
        std::fs::read_to_string(ruta).map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
    let palabras =
        leer_palabras(&texto, maquina.bits_palabra).map_err(|e| format!("{ruta}: {e}"))?;
    Ok(vec![(inicio, palabras)])
}
