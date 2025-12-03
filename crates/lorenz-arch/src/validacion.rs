//! Las reglas que una descripcion debe cumplir antes de convertirse en una maquina ejecutable.
//!
//! El cargador acumula todos los fallos y los devuelve juntos: ver los diez problemas de una vez
//! ahorra diez ciclos de prueba y error.
//!
//! Cada funcion de este modulo corresponde a una regla de [`Regla`] y cita el requisito de la ERS
//! que la exige.

use std::collections::BTreeSet;

use crate::error::{FalloValidacion, Regla};
use crate::modelo::*;

/// Aplica todas las reglas y devuelve los fallos encontrados, en orden de regla.
pub fn validar(m: &Maquina) -> Vec<FalloValidacion> {
    let mut fallos = Vec::new();
    anchos_de_palabra(m, &mut fallos);
    codificaciones_ambiguas(m, &mut fallos);
    campos_y_operandos(m, &mut fallos);
    sintaxis_de_instrucciones(m, &mut fallos);
    roles_de_registro(m, &mut fallos);
    pares_de_registros(m, &mut fallos);
    nombres_duplicados(m, &mut fallos);
    regiones(m, &mut fallos);
    arranque(m, &mut fallos);
    dispositivos_citados(m, &mut fallos);
    banderas_citadas(m, &mut fallos);
    nombres_de_semantica(m, &mut fallos);
    rangos(m, &mut fallos);
    fallos
}

/// RF-ARQ-013: la sintaxis de cada instruccion empieza por su mnemonico y menciona exactamente
/// sus operandos, cada uno una vez. Sin esto el ensamblador no podria leerla ni el desensamblador
/// escribirla.
fn sintaxis_de_instrucciones(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for ins in &m.instrucciones {
        let plantilla = match crate::plantilla::Plantilla::analizar(&ins.sintaxis) {
            Ok(p) => p,
            Err(e) => {
                fallos.push(fallo(
                    Regla::SintaxisInvalida,
                    format!("la sintaxis de '{}' no es valida: {}", ins.mnemonico, e),
                ));
                continue;
            }
        };
        if !plantilla.mnemonico().eq_ignore_ascii_case(&ins.mnemonico) {
            fallos.push(fallo(
                Regla::SintaxisInvalida,
                format!(
                    "la sintaxis de '{}' empieza por '{}': debe empezar por su mnemonico",
                    ins.mnemonico,
                    plantilla.mnemonico()
                ),
            ));
        }
        let huecos = plantilla.huecos();
        let mut vistos: BTreeSet<&str> = BTreeSet::new();
        for h in &huecos {
            if !vistos.insert(h) {
                fallos.push(fallo(
                    Regla::SintaxisInvalida,
                    format!(
                        "la sintaxis de '{}' repite el operando '{}'",
                        ins.mnemonico, h
                    ),
                ));
            }
            if !ins.operandos.iter().any(|o| o.nombre == *h) {
                fallos.push(fallo(
                    Regla::SintaxisInvalida,
                    format!(
                        "la sintaxis de '{}' cita '{{{}}}', que no es un operando suyo",
                        ins.mnemonico, h
                    ),
                ));
            }
        }
        for o in &ins.operandos {
            if !vistos.contains(o.nombre.as_str()) {
                fallos.push(fallo(
                    Regla::SintaxisInvalida,
                    format!(
                        "el operando '{}' de '{}' no aparece en su sintaxis",
                        o.nombre, ins.mnemonico
                    ),
                ));
            }
        }
    }
}

fn fallo(regla: Regla, detalle: String) -> FalloValidacion {
    FalloValidacion {
        regla,
        detalle,
        posicion: None,
    }
}

/// RF-ARQ-016: los bits de cada palabra suman el ancho de palabra declarado.
fn anchos_de_palabra(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for ins in &m.instrucciones {
        for (indice, palabra) in ins.codificacion.palabras().iter().enumerate() {
            if palabra.ancho() != m.bits_palabra {
                fallos.push(fallo(
                    Regla::SumaDeBits,
                    format!(
                        "la palabra {} de '{}' suma {} bits y la maquina declara palabras de {} bits",
                        indice + 1,
                        ins.mnemonico,
                        palabra.ancho(),
                        m.bits_palabra
                    ),
                ));
            }
        }
    }
}

/// RF-ARQ-017: dos instrucciones no pueden admitir la misma codificacion binaria.
fn codificaciones_ambiguas(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for (i, a) in m.instrucciones.iter().enumerate() {
        for b in m.instrucciones.iter().skip(i + 1) {
            if a.codificacion.es_ambigua_con(&b.codificacion) {
                fallos.push(fallo(
                    Regla::CodificacionAmbigua,
                    format!(
                        "'{}' ({}) y '{}' ({}) aceptan la misma palabra; el desensamblador no podria distinguirlas",
                        a.mnemonico,
                        a.codificacion.texto(),
                        b.mnemonico,
                        b.codificacion.texto()
                    ),
                ));
            }
        }
    }
}

/// RF-ARQ-018: cada campo de la codificacion tiene operando declarado y viceversa.
fn campos_y_operandos(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for ins in &m.instrucciones {
        let campos: BTreeSet<&str> = ins.codificacion.campos().into_iter().collect();
        let operandos: BTreeSet<&str> = ins.operandos.iter().map(|o| o.nombre.as_str()).collect();

        for campo in campos.difference(&operandos) {
            fallos.push(fallo(
                Regla::CampoSinOperando,
                format!(
                    "el campo '{}' de '{}' no tiene operando declarado",
                    campo, ins.mnemonico
                ),
            ));
        }
        for operando in operandos.difference(&campos) {
            fallos.push(fallo(
                Regla::CampoSinOperando,
                format!(
                    "el operando '{}' de '{}' no aparece en su codificacion",
                    operando, ins.mnemonico
                ),
            ));
        }

        for operando in &ins.operandos {
            if let TipoOperando::Registro { permitidos } = &operando.tipo {
                for nombre in permitidos {
                    if !m.es_registro_o_par(nombre) {
                        fallos.push(fallo(
                            Regla::CampoSinOperando,
                            format!(
                                "el operando '{}' de '{}' admite el registro '{}', que no esta declarado",
                                operando.nombre, ins.mnemonico, nombre
                            ),
                        ));
                    }
                }
                let bits_campo = ins.codificacion.bits_totales(&operando.nombre);
                if bits_campo > 0 {
                    let capacidad = 1usize << bits_campo.min(32);
                    if permitidos.len() > capacidad {
                        fallos.push(fallo(
                            Regla::CampoSinOperando,
                            format!(
                                "el operando '{}' de '{}' admite {} registros y su campo solo codifica {}",
                                operando.nombre,
                                ins.mnemonico,
                                permitidos.len(),
                                capacidad
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// RF-ARQ-009: los roles unicos aparecen exactamente una vez.
fn roles_de_registro(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    let contar = |rol: RolRegistro| m.registros.iter().filter(|r| r.rol == rol).count();

    if contar(RolRegistro::ContadorPrograma) == 0 {
        fallos.push(fallo(
            Regla::RolUnico,
            "ningun registro tiene el rol de contador de programa".to_string(),
        ));
    }

    for rol in [
        RolRegistro::ContadorPrograma,
        RolRegistro::PunteroPila,
        RolRegistro::Banderas,
    ] {
        let n = contar(rol);
        if n > 1 {
            let nombres: Vec<&str> = m
                .registros
                .iter()
                .filter(|r| r.rol == rol)
                .map(|r| r.nombre.as_str())
                .collect();
            fallos.push(fallo(
                Regla::RolUnico,
                format!(
                    "el rol {:?} lo declaran {} registros: {}",
                    rol,
                    n,
                    nombres.join(", ")
                ),
            ));
        }
    }

    if !m.banderas.is_empty() && contar(RolRegistro::Banderas) == 0 {
        fallos.push(fallo(
            Regla::RolUnico,
            "se declaran banderas pero ningun registro tiene el rol de banderas".to_string(),
        ));
    }
}

/// RF-ARQ-007: nombres unicos dentro de cada familia.
fn nombres_duplicados(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    let mut revisar = |familia: &str, nombres: Vec<&str>| {
        let mut vistos = BTreeSet::new();
        for nombre in nombres {
            if !vistos.insert(nombre) {
                fallos.push(fallo(
                    Regla::NombreDuplicado,
                    format!("hay dos {familia} con el nombre '{nombre}'"),
                ));
            }
        }
    };

    revisar(
        "registros o pares de registros",
        m.registros
            .iter()
            .map(|r| r.nombre.as_str())
            .chain(m.pares.iter().map(|p| p.nombre.as_str()))
            .collect(),
    );
    revisar(
        "regiones",
        m.regiones.iter().map(|r| r.nombre.as_str()).collect(),
    );
    revisar(
        "dispositivos",
        m.dispositivos.iter().map(|d| d.nombre.as_str()).collect(),
    );
    revisar(
        "instrucciones",
        m.instrucciones
            .iter()
            .map(|i| i.mnemonico.as_str())
            .collect(),
    );
}

/// RF-ARQ-010: un par junta dos registros generales distintos y declarados, y su ancho cabe en 64
/// bits.
fn pares_de_registros(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for par in &m.pares {
        let mut bits = 0;
        for (parte, nombre) in [("alto", &par.alto), ("bajo", &par.bajo)] {
            match m.registro(nombre) {
                None => fallos.push(fallo(
                    Regla::ParInvalido,
                    format!(
                        "el registro {parte} '{nombre}' del par '{}' no esta declarado",
                        par.nombre
                    ),
                )),
                Some(r) if r.rol != RolRegistro::General => fallos.push(fallo(
                    Regla::ParInvalido,
                    format!(
                        "el registro {parte} '{nombre}' del par '{}' no es de uso general",
                        par.nombre
                    ),
                )),
                Some(r) => bits += r.bits,
            }
        }
        if par.alto == par.bajo {
            fallos.push(fallo(
                Regla::ParInvalido,
                format!(
                    "el par '{}' usa dos veces el registro '{}'",
                    par.nombre, par.alto
                ),
            ));
        }
        if bits > 64 {
            fallos.push(fallo(
                Regla::ParInvalido,
                format!(
                    "el par '{}' suma {bits} bits y el maximo admitido es 64",
                    par.nombre
                ),
            ));
        }
    }
}

/// RF-ARQ-027: las regiones de memoria no se solapan.
fn regiones(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for (i, a) in m.regiones.iter().enumerate() {
        if a.desde > a.hasta {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!(
                    "la region '{}' empieza en {:#x} y termina en {:#x}: el final va antes del inicio",
                    a.nombre, a.desde, a.hasta
                ),
            ));
            continue;
        }
        if a.hasta > m.direccion_maxima() {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!(
                    "la region '{}' llega a {:#x}, fuera del espacio de {} bits de direccion",
                    a.nombre, a.hasta, m.bits_direccion
                ),
            ));
        }
        for b in m.regiones.iter().skip(i + 1) {
            if a.se_solapa_con(b) {
                fallos.push(fallo(
                    Regla::RegionSolapada,
                    format!(
                        "las regiones '{}' ({:#x}..{:#x}) y '{}' ({:#x}..{:#x}) se solapan",
                        a.nombre, a.desde, a.hasta, b.nombre, b.desde, b.hasta
                    ),
                ));
            }
        }
    }
}

/// RF-ARQ-028: la direccion de arranque cae dentro de una region declarada.
fn arranque(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    if m.regiones.is_empty() {
        return;
    }
    if m.region_de(m.arranque).is_none() {
        fallos.push(fallo(
            Regla::ArranqueFueraDeRegion,
            format!(
                "la direccion de arranque {:#x} no cae en ninguna region declarada",
                m.arranque
            ),
        ));
    }
}

/// RF-ARQ-031: toda region que nombra un dispositivo lo tiene declarado.
fn dispositivos_citados(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for region in &m.regiones {
        if let Some(nombre) = &region.dispositivo {
            if m.dispositivo(nombre).is_none() {
                fallos.push(fallo(
                    Regla::DispositivoInexistente,
                    format!(
                        "la region '{}' responde al dispositivo '{}', que no esta declarado",
                        region.nombre, nombre
                    ),
                ));
            }
        }
    }

    for dispositivo in &m.dispositivos {
        if let TipoDispositivo::Temporizador { periodo: 0 } = dispositivo.tipo {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!(
                    "el temporizador '{}' tiene periodo 0; debe ser de al menos 1 ciclo",
                    dispositivo.nombre
                ),
            ));
        }
    }

    for dispositivo in &m.dispositivos {
        let TipoDispositivo::Generico { programa, .. } = &dispositivo.tipo else {
            continue;
        };
        let mut permitidos: Vec<&str> = vec!["valor", "puerto"];
        permitidos.extend(dispositivo.puertos.keys().map(String::as_str));
        let mut asignables: Vec<&str> = dispositivo.puertos.keys().map(String::as_str).collect();
        if dispositivo.interrupcion.is_some() {
            permitidos.push("interrumpir");
            asignables.push("interrumpir");
        }
        for nombre in programa.nombres_leidos() {
            if !permitidos.contains(&nombre.as_str()) {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "el script de '{}' cita '{}', que no es valor, puerto, uno de sus puertos ni interrumpir",
                        dispositivo.nombre, nombre
                    ),
                ));
            }
        }
        for nombre in programa.nombres_escritos() {
            if !asignables.contains(&nombre.as_str()) {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "el script de '{}' asigna a '{}', que no es uno de sus puertos ni interrumpir (este ultimo exige declarar una interrupcion)",
                        dispositivo.nombre, nombre
                    ),
                ));
            }
        }
        if programa.accede_a_memoria_o_puertos() || programa.puede_detener() {
            fallos.push(fallo(
                Regla::SemanticaInvalida,
                format!(
                    "el script de '{}' no puede usar mem, in, out ni detener",
                    dispositivo.nombre
                ),
            ));
        }
    }

    for dispositivo in &m.dispositivos {
        if let TipoDispositivo::Bloques {
            tamano_bloque,
            cantidad_bloques,
        } = dispositivo.tipo
        {
            if tamano_bloque == 0 || cantidad_bloques == 0 {
                fallos.push(fallo(
                    Regla::ValorFueraDeRango,
                    format!(
                        "el dispositivo de bloques '{}' necesita tamano_bloque y cantidad_bloques de al menos 1",
                        dispositivo.nombre
                    ),
                ));
            }
        }
    }

    if let Some(int) = &m.interrupciones {
        for linea in int.prioridades.keys() {
            if !int.vectores.contains_key(linea) {
                fallos.push(fallo(
                    Regla::ValorFueraDeRango,
                    format!("la linea {linea} tiene prioridad pero no tiene vector declarado"),
                ));
            }
        }
        for dispositivo in &m.dispositivos {
            if let Some(linea) = dispositivo.interrupcion {
                if !int.vectores.contains_key(&linea) {
                    fallos.push(fallo(
                        Regla::DispositivoInexistente,
                        format!(
                            "el dispositivo '{}' levanta la linea {} y no hay vector declarado para ella",
                            dispositivo.nombre, linea
                        ),
                    ));
                }
            }
        }
    }
}

/// RF-ARQ-024: toda bandera citada por una instruccion esta declarada.
fn banderas_citadas(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for ins in &m.instrucciones {
        for nombre in &ins.afecta {
            if m.bandera(nombre).is_none() {
                fallos.push(fallo(
                    Regla::BanderaInexistente,
                    format!(
                        "'{}' dice modificar la bandera '{}', que no esta declarada",
                        ins.mnemonico, nombre
                    ),
                ));
            }
        }
    }
}

/// RF-ARQ-003, RF-ARQ-004: valores dentro de los limites del formato.
fn rangos(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    if !(4..=64).contains(&m.bits_palabra) {
        fallos.push(fallo(
            Regla::ValorFueraDeRango,
            format!(
                "el ancho de palabra es {} y debe estar entre 4 y 64 bits",
                m.bits_palabra
            ),
        ));
    }
    if !(8..=64).contains(&m.bits_direccion) {
        fallos.push(fallo(
            Regla::ValorFueraDeRango,
            format!(
                "el ancho de direccion es {} y debe estar entre 8 y 64 bits",
                m.bits_direccion
            ),
        ));
    }

    for registro in &m.registros {
        if registro.bits == 0 || registro.bits > 64 {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!(
                    "el registro '{}' declara {} bits; el rango es 1..64",
                    registro.nombre, registro.bits
                ),
            ));
            continue;
        }
        let maximo = if registro.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << registro.bits) - 1
        };
        if registro.inicial > maximo {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!(
                    "el valor inicial {:#x} del registro '{}' no cabe en sus {} bits",
                    registro.inicial, registro.nombre, registro.bits
                ),
            ));
        }
    }

    if let Some(banderas_reg) = m.registro_con_rol(RolRegistro::Banderas) {
        for bandera in &m.banderas {
            if bandera.bit >= banderas_reg.bits {
                fallos.push(fallo(
                    Regla::ValorFueraDeRango,
                    format!(
                        "la bandera '{}' ocupa el bit {} y el registro '{}' solo tiene {} bits",
                        bandera.nombre, bandera.bit, banderas_reg.nombre, banderas_reg.bits
                    ),
                ));
            }
        }
    }

    let mut bits_vistos: BTreeSet<u32> = BTreeSet::new();
    for bandera in &m.banderas {
        if !bits_vistos.insert(bandera.bit) {
            fallos.push(fallo(
                Regla::NombreDuplicado,
                format!("dos banderas ocupan el bit {}", bandera.bit),
            ));
        }
    }

    if m.arranque > m.direccion_maxima() {
        fallos.push(fallo(
            Regla::ValorFueraDeRango,
            format!(
                "la direccion de arranque {:#x} no cabe en {} bits de direccion",
                m.arranque, m.bits_direccion
            ),
        ));
    }
}

/// RF-ARQ-022: la semantica solo cita nombres que existen y solo asigna a lo asignable.
///
/// Un nombre puede ser un operando de la instruccion, un registro, una bandera, los alias `pc` y
/// `sp` cuando existe un registro con ese rol, o `ie` (interrupciones habilitadas) cuando la
/// maquina declara interrupciones.
fn nombres_de_semantica(m: &Maquina, fallos: &mut Vec<FalloValidacion>) {
    for ins in &m.instrucciones {
        let es_operando = |n: &str| ins.operandos.iter().any(|o| o.nombre == n);
        let es_alias = |n: &str| match n {
            "pc" => m.registro_con_rol(RolRegistro::ContadorPrograma).is_some(),
            "sp" => m.registro_con_rol(RolRegistro::PunteroPila).is_some(),
            "ie" => m.interrupciones.is_some(),
            _ => false,
        };
        let existe = |n: &str| {
            es_operando(n) || m.es_registro_o_par(n) || m.bandera(n).is_some() || es_alias(n)
        };

        for nombre in ins.programa.nombres_leidos() {
            if !existe(&nombre) {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "la semantica de '{}' cita '{}', que no es un operando, un registro ni una bandera",
                        ins.mnemonico, nombre
                    ),
                ));
            }
        }

        for nombre in ins.programa.nombres_escritos() {
            if !existe(&nombre) {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "la semantica de '{}' asigna a '{}', que no existe",
                        ins.mnemonico, nombre
                    ),
                ));
                continue;
            }

            let operando_no_registro = ins
                .operandos
                .iter()
                .find(|o| o.nombre == nombre)
                .map(|o| !matches!(o.tipo, TipoOperando::Registro { .. }))
                .unwrap_or(false);
            if operando_no_registro {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "la semantica de '{}' asigna al operando '{}', que no es un registro",
                        ins.mnemonico, nombre
                    ),
                ));
            }

            if m.registro(&nombre)
                .map(|r| r.rol == RolRegistro::Fijo)
                .unwrap_or(false)
            {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!(
                        "la semantica de '{}' asigna al registro '{}', que es de solo lectura",
                        ins.mnemonico, nombre
                    ),
                ));
            }
        }
    }
}
