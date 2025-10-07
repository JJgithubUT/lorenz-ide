//! Guardar un fallo como archivo y reproducirlo en otra instalacion (RF-DEP-030, RF-DEP-031).
//!
//! El archivo contiene el estado inicial (registros y memoria), todo lo que el usuario hizo a mano
//! durante la grabacion y cuantas instrucciones se ejecutaron. Con eso y la misma descripcion de
//! arquitectura, el emulador (que es determinista) llega al mismo estado. Para comprobarlo se
//! guarda tambien el estado final.
//!
//! Limite de la version 0.1: solo se guardan registros y memoria. Las colas de los puertos, los
//! temporizadores y los dispositivos de bloques deben partir del estado inicial de la maquina.

use lorenz_arch::modelo::Maquina;
use lorenz_emu::Emulador;
use serde::{Deserialize, Serialize};

use crate::depurador::Depurador;
use crate::error::ErrorDepurador;
use crate::traza::Intervencion;

pub const FORMATO_FALLO: &str = "lorenz-fallo-0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RegistroToml {
    nombre: String,
    valor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TramoToml {
    direccion: String,
    palabras: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct IntervencionToml {
    instante: u64,
    tipo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nombre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    numero: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    direccion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    valor: Option<String>,
    /// Bytes en hexadecimal, sin separadores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bytes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FinalToml {
    registros: Vec<RegistroToml>,
    suma_memoria: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Archivo {
    formato: String,
    maquina: String,
    /// Instrucciones que se ejecutan desde el estado inicial.
    pasos: u64,
    /// Texto del fallo que ocurre al ejecutar una instruccion mas, si lo hubo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fallo: Option<String>,
    inicial: Vec<RegistroToml>,
    #[serde(default, rename = "memoria")]
    memoria: Vec<TramoToml>,
    #[serde(default, rename = "intervencion")]
    intervenciones: Vec<IntervencionToml>,
    #[serde(rename = "final")]
    final_: FinalToml,
}

fn hex(n: u64) -> String {
    format!("{n:#x}")
}

fn de_hex(t: &str) -> Result<u64, String> {
    let t = t.trim();
    let r = match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse(),
    };
    r.map_err(|_| format!("`{t}` no es un numero"))
}

/// Un fallo capturado, listo para guardarse o reproducirse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallo {
    archivo: Archivo,
}

/// Resultado de reproducir un fallo.
pub struct Reproduccion {
    /// `true` si el estado final y el fallo coinciden con los guardados.
    pub identica: bool,
    /// Que no coincide, si algo.
    pub diferencias: Vec<String>,
    /// La maquina tras reproducir, para seguir depurando desde ahi.
    pub emulador: Emulador,
}

impl Fallo {
    /// Captura la traza actual del depurador como un fallo reproducible.
    ///
    /// `fallo` es el texto del error que ocurre al ejecutar la siguiente instruccion, si se quiere
    /// que la reproduccion lo compruebe.
    pub fn capturar(d: &mut Depurador, fallo: Option<String>) -> Result<Fallo, ErrorDepurador> {
        let origen = d.instante();
        let t = d.traza().ok_or(ErrorDepurador::SinTraza)?;
        let pasos = t.longitud();
        let inicial = t.instantanea_hasta(0).1.clone();
        let intervenciones: Vec<IntervencionToml> = t
            .intervenciones()
            .iter()
            .map(|(k, i)| a_toml_intervencion(*k, i))
            .collect();
        let maquina = d.maquina().clone();

        let est = inicial.estado();
        let registros_ini: Vec<RegistroToml> = est
            .nombres()
            .iter()
            .filter_map(|n| {
                est.registro(n).ok().map(|v| RegistroToml {
                    nombre: n.clone(),
                    valor: hex(v),
                })
            })
            .collect();
        let mut memoria = Vec::new();
        for r in &maquina.regiones {
            let mut tramo: Option<(u64, Vec<String>)> = None;
            for dir in r.desde..=r.hasta {
                let v = inicial.memoria().leer_privilegiado(dir).unwrap_or(0);
                match (&mut tramo, v) {
                    (Some((_, ps)), v) if v != 0 || ps.last().is_some_and(|u| u != "0x0") => {
                        ps.push(hex(v))
                    }
                    (None, v) if v != 0 => tramo = Some((dir, vec![hex(v)])),
                    (Some(_), _) => {
                        if let Some((d0, ps)) = tramo.take() {
                            memoria.push(cerrar(d0, ps));
                        }
                    }
                    _ => {}
                }
            }
            if let Some((d0, ps)) = tramo.take() {
                memoria.push(cerrar(d0, ps));
            }
        }

        // El estado final es el del extremo de la traza.
        d.ir_a(pasos)?;
        let e = d.emulador().estado();
        let finales: Vec<RegistroToml> = e
            .nombres()
            .iter()
            .filter_map(|n| {
                e.registro(n).ok().map(|v| RegistroToml {
                    nombre: n.clone(),
                    valor: hex(v),
                })
            })
            .collect();
        let suma = d.emulador().memoria().suma_de_verificacion();
        d.ir_a(origen)?;

        Ok(Fallo {
            archivo: Archivo {
                formato: FORMATO_FALLO.to_string(),
                maquina: maquina.nombre.clone(),
                pasos,
                fallo,
                inicial: registros_ini,
                memoria,
                intervenciones,
                final_: FinalToml {
                    registros: finales,
                    suma_memoria: hex(suma),
                },
            },
        })
    }

    pub fn a_toml(&self) -> String {
        toml::to_string(&self.archivo).expect("el archivo de fallo siempre se puede serializar")
    }

    pub fn desde_toml(texto: &str) -> Result<Fallo, String> {
        let a: Archivo =
            toml::from_str(texto).map_err(|e| format!("archivo de fallo invalido: {e}"))?;
        if a.formato != FORMATO_FALLO {
            return Err(format!(
                "formato `{}` no reconocido; esta version lee `{FORMATO_FALLO}`",
                a.formato
            ));
        }
        for i in &a.intervenciones {
            de_toml_intervencion(i)?;
        }
        for r in a.inicial.iter().chain(&a.final_.registros) {
            de_hex(&r.valor)?;
        }
        de_hex(&a.final_.suma_memoria)?;
        Ok(Fallo { archivo: a })
    }

    pub fn maquina(&self) -> &str {
        &self.archivo.maquina
    }

    pub fn pasos(&self) -> u64 {
        self.archivo.pasos
    }

    pub fn fallo(&self) -> Option<&str> {
        self.archivo.fallo.as_deref()
    }

    /// Rehace la ejecucion sobre `maquina` y comprueba que el resultado es el guardado.
    pub fn reproducir(&self, maquina: &Maquina) -> Result<Reproduccion, ErrorDepurador> {
        let a = &self.archivo;
        if maquina.nombre != a.maquina {
            return Err(ErrorDepurador::Reproduccion(format!(
                "el fallo es de la maquina `{}` y esta es `{}`",
                a.maquina, maquina.nombre
            )));
        }
        let mut emu = Emulador::nuevo(maquina.clone())?;
        let malo = |e: String| ErrorDepurador::Reproduccion(e);
        for t in &a.memoria {
            let dir = de_hex(&t.direccion).map_err(malo)?;
            let palabras: Vec<u64> = t
                .palabras
                .iter()
                .map(|p| de_hex(p))
                .collect::<Result<_, _>>()
                .map_err(malo)?;
            emu.cargar_programa(dir, &palabras)?;
        }
        for r in &a.inicial {
            let v = de_hex(&r.valor).map_err(malo)?;
            let e = emu.estado_mut();
            if r.nombre.eq_ignore_ascii_case(e.nombre_del_pc()) {
                e.fijar_pc(v);
            } else if e
                .nombre_del_sp()
                .is_some_and(|s| s.eq_ignore_ascii_case(&r.nombre))
            {
                e.fijar_sp(v);
            } else {
                // Los registros de solo lectura ya valen lo que declara la arquitectura.
                let _ = e.fijar_registro(&r.nombre, v);
            }
        }
        let mut d = Depurador::nuevo(emu);
        d.grabar(true);
        let mut intervenciones: Vec<(u64, Intervencion)> = a
            .intervenciones
            .iter()
            .map(|i| de_toml_intervencion(i).map_err(malo))
            .collect::<Result<_, _>>()?;
        intervenciones.sort_by_key(|(k, _)| *k);
        let mut siguiente = 0;
        for k in 0..=a.pasos {
            while siguiente < intervenciones.len() && intervenciones[siguiente].0 == k {
                d.aplicar_a_mano(&intervenciones[siguiente].1)?;
                siguiente += 1;
            }
            if k < a.pasos {
                d.paso(1);
            }
        }
        let mut diferencias = Vec::new();
        let emu = d.emulador();
        for r in &a.final_.registros {
            let esperado = de_hex(&r.valor).map_err(malo)?;
            match emu.estado().registro(&r.nombre) {
                Ok(v) if v == esperado => {}
                Ok(v) => diferencias.push(format!(
                    "el registro {} vale {v:#x} y debia valer {esperado:#x}",
                    r.nombre
                )),
                Err(_) => diferencias.push(format!("la maquina no tiene el registro {}", r.nombre)),
            }
        }
        let esperada = de_hex(&a.final_.suma_memoria).map_err(malo)?;
        let suma = emu.memoria().suma_de_verificacion();
        if suma != esperada {
            diferencias.push(format!(
                "la memoria difiere: suma {suma:#x} y debia ser {esperada:#x}"
            ));
        }
        let mut emulador = d.emulador().clone();
        if let Some(esperado) = &a.fallo {
            match emulador.paso() {
                Err(e) if &e.to_string() == esperado => {}
                Err(e) => {
                    diferencias.push(format!("el fallo es otro: `{e}` y debia ser `{esperado}`"))
                }
                Ok(_) => diferencias.push(format!("no hubo fallo y debia ocurrir `{esperado}`")),
            }
        }
        Ok(Reproduccion {
            identica: diferencias.is_empty(),
            diferencias,
            emulador,
        })
    }
}

fn cerrar(direccion: u64, mut palabras: Vec<String>) -> TramoToml {
    while palabras.last().is_some_and(|p| p == "0x0") {
        palabras.pop();
    }
    TramoToml {
        direccion: hex(direccion),
        palabras,
    }
}

fn a_toml_intervencion(instante: u64, i: &Intervencion) -> IntervencionToml {
    let vacia = |tipo: &str| IntervencionToml {
        instante,
        tipo: tipo.to_string(),
        nombre: None,
        numero: None,
        direccion: None,
        valor: None,
        bytes: None,
    };
    match i {
        Intervencion::Registro { nombre, valor } => IntervencionToml {
            nombre: Some(nombre.clone()),
            valor: Some(hex(*valor)),
            ..vacia("registro")
        },
        Intervencion::Bandera { nombre, encendida } => IntervencionToml {
            nombre: Some(nombre.clone()),
            valor: Some(u8::from(*encendida).to_string()),
            ..vacia("bandera")
        },
        Intervencion::Memoria { direccion, valor } => IntervencionToml {
            direccion: Some(hex(*direccion)),
            valor: Some(hex(*valor)),
            ..vacia("memoria")
        },
        Intervencion::Puerto { numero, valor } => IntervencionToml {
            numero: Some(hex(*numero)),
            valor: Some(hex(*valor)),
            ..vacia("puerto")
        },
        Intervencion::Entrada { numero, valor } => IntervencionToml {
            numero: Some(hex(*numero)),
            valor: Some(hex(*valor)),
            ..vacia("entrada")
        },
        Intervencion::LecturaDePuerto { numero } => IntervencionToml {
            numero: Some(hex(*numero)),
            ..vacia("lectura")
        },
        Intervencion::Tecla {
            dispositivo,
            codigo,
        } => IntervencionToml {
            nombre: Some(dispositivo.clone()),
            valor: Some(hex(*codigo)),
            ..vacia("tecla")
        },
        Intervencion::Serie { dispositivo, bytes } => IntervencionToml {
            nombre: Some(dispositivo.clone()),
            bytes: Some(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
            ..vacia("serie")
        },
        Intervencion::Interrupcion { linea } => IntervencionToml {
            numero: Some(hex(u64::from(*linea))),
            ..vacia("interrupcion")
        },
    }
}

fn de_toml_intervencion(i: &IntervencionToml) -> Result<(u64, Intervencion), String> {
    let falta = |c: &str| format!("a la intervencion `{}` le falta `{c}`", i.tipo);
    let nombre = || i.nombre.clone().ok_or_else(|| falta("nombre"));
    let numero = || de_hex(i.numero.as_deref().ok_or_else(|| falta("numero"))?);
    let direccion = || de_hex(i.direccion.as_deref().ok_or_else(|| falta("direccion"))?);
    let valor = || de_hex(i.valor.as_deref().ok_or_else(|| falta("valor"))?);
    let que = match i.tipo.as_str() {
        "registro" => Intervencion::Registro {
            nombre: nombre()?,
            valor: valor()?,
        },
        "bandera" => Intervencion::Bandera {
            nombre: nombre()?,
            encendida: valor()? != 0,
        },
        "memoria" => Intervencion::Memoria {
            direccion: direccion()?,
            valor: valor()?,
        },
        "puerto" => Intervencion::Puerto {
            numero: numero()?,
            valor: valor()?,
        },
        "entrada" => Intervencion::Entrada {
            numero: numero()?,
            valor: valor()?,
        },
        "lectura" => Intervencion::LecturaDePuerto { numero: numero()? },
        "tecla" => Intervencion::Tecla {
            dispositivo: nombre()?,
            codigo: valor()?,
        },
        "serie" => {
            let hexa = i.bytes.clone().ok_or_else(|| falta("bytes"))?;
            if hexa.len() % 2 != 0 || !hexa.is_ascii() {
                return Err(format!("`{hexa}` no son bytes en hexadecimal"));
            }
            let bytes = (0..hexa.len() / 2)
                .map(|k| {
                    u8::from_str_radix(&hexa[2 * k..2 * k + 2], 16)
                        .map_err(|_| format!("`{hexa}` no son bytes en hexadecimal"))
                })
                .collect::<Result<Vec<u8>, String>>()?;
            Intervencion::Serie {
                dispositivo: nombre()?,
                bytes,
            }
        }
        "interrupcion" => Intervencion::Interrupcion {
            linea: u32::try_from(numero()?).map_err(|_| "linea demasiado grande".to_string())?,
        },
        otro => return Err(format!("intervencion `{otro}` desconocida")),
    };
    Ok((i.instante, que))
}
