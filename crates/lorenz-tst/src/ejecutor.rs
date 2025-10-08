//! Ejecutar una prueba de proyecto y decir si pasa (RF-TST-002 a RF-TST-010, RF-TST-013,
//! RF-TST-014).

use std::time::Instant;

use lorenz_arch::modelo::TipoDispositivo;
use lorenz_dbg::vistas;
use lorenz_dbg::{Depurador, Parada};
use lorenz_emu::{Emulador, ErrorEjecucion};
use lorenz_ens::ensamblador::Ensamblador;
use lorenz_ens::preprocesar::CargadorDeDisco;
use serde::{Deserialize, Serialize};

use crate::formato::{Evento, Momento, Proyecto, Prueba};

/// Extensiones que se ensamblan; cualquier otra se lee como hexadecimal.
pub const EXTENSIONES_FUENTE: [&str; 3] = ["s", "asm", "lasm"];

/// Cuanto se deja crecer la traza cuando hay que localizar un fallo.
const MAX_PASOS_DE_TRAZA: u64 = 5_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Veredicto {
    Pasa,
    /// Alguna afirmacion no se cumplio.
    Falla,
    /// La prueba no se pudo ni preparar (el programa no ensambla, no existe...).
    Error,
}

/// La instruccion donde se origino una divergencia (RF-TST-013).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Donde {
    /// Instante de la ejecucion, contado en instrucciones desde el principio.
    pub instante: u64,
    pub direccion: u64,
    pub instruccion: String,
    pub fuente: Option<(String, usize)>,
}

/// Una afirmacion que no se cumplio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fallo {
    /// Que se comprobaba, por ejemplo `registro A`.
    pub que: String,
    pub esperado: String,
    pub obtenido: String,
    /// La ultima instruccion que toco lo comprobado, o donde termino la ejecucion.
    #[serde(default)]
    pub donde: Option<Donde>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resultado {
    pub nombre: String,
    pub etiquetas: Vec<String>,
    pub veredicto: Veredicto,
    pub fallos: Vec<Fallo>,
    /// Por que no se pudo ejecutar, si `veredicto` es `Error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motivo: Option<String>,
    pub instrucciones: u64,
    pub ciclos: u64,
    pub milisegundos: u64,
}

impl Resultado {
    pub fn paso(&self) -> bool {
        self.veredicto == Veredicto::Pasa
    }
}

fn es_fuente(ruta: &std::path::Path) -> bool {
    ruta.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONES_FUENTE.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Deja la maquina lista: programa cargado y estado inicial puesto.
fn preparar(proyecto: &Proyecto, prueba: &Prueba, con_traza: bool) -> Result<Depurador, String> {
    let m = &proyecto.maquina;
    let ruta = prueba.programa.to_string_lossy().into_owned();
    let mut emu = Emulador::nuevo(m.clone()).map_err(|e| e.to_string())?;
    let mut objeto = None;
    if es_fuente(&prueba.programa) {
        let texto = std::fs::read_to_string(&prueba.programa)
            .map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
        let raiz = lorenz_ens::ensamblador::Fuente::nueva(ruta.clone(), texto);
        let ens = Ensamblador::nuevo(m).map_err(|e| format!("{ruta}: {e}"))?;
        let (o, fuentes) = ens.ensamblar_con(&raiz, &CargadorDeDisco).map_err(|es| {
            es.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        for s in &o.segmentos {
            emu.cargar_programa(s.direccion, &s.palabras)
                .map_err(|e| format!("{ruta}: {e}"))?;
        }
        objeto = Some((o, fuentes));
    } else {
        let texto = std::fs::read_to_string(&prueba.programa)
            .map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
        let palabras = lorenz_emu::programa::leer_palabras(&texto, m.bits_palabra)
            .map_err(|e| format!("{ruta}: {e}"))?;
        emu.cargar_programa(m.arranque, &palabras)
            .map_err(|e| format!("{ruta}: {e}"))?;
    }
    let ini = &prueba.def.inicial;
    for t in &ini.memoria {
        let palabras: Vec<u64> = t.palabras.iter().map(|p| p.0).collect();
        emu.cargar_programa(t.direccion.0, &palabras)
            .map_err(|e| e.to_string())?;
    }
    for (nombre, v) in &ini.registros {
        let e = emu.estado_mut();
        if nombre.eq_ignore_ascii_case(e.nombre_del_pc()) {
            e.fijar_pc(v.0);
        } else if e
            .nombre_del_sp()
            .is_some_and(|s| s.eq_ignore_ascii_case(nombre))
        {
            e.fijar_sp(v.0);
        } else {
            e.fijar_registro(nombre, v.0).map_err(|e| e.to_string())?;
        }
    }
    for (nombre, v) in &ini.banderas {
        emu.estado_mut()
            .fijar_bandera(nombre, *v)
            .map_err(|e| e.to_string())?;
    }
    let mut d = Depurador::nuevo(emu);
    if let Some((o, fuentes)) = &objeto {
        let pares: Vec<(String, String)> = fuentes
            .iter()
            .map(|f| (f.nombre.clone(), f.texto.clone()))
            .collect();
        d.con_fuente(o, &pares);
    }
    if con_traza {
        d.grabar(true);
        let tope = (prueba.limite() + 1).min(MAX_PASOS_DE_TRAZA) as usize;
        if let Some(t) = d.traza_mut() {
            t.fijar_limite(tope);
        }
    }
    Ok(d)
}

/// Como termino la ejecucion.
#[derive(Debug)]
struct Corrida {
    error: Option<(ErrorEjecucion, u64)>,
    limite_alcanzado: bool,
}

fn aplicar(d: &mut Depurador, e: &Evento) -> Result<(), String> {
    match e {
        Evento::Tecla {
            dispositivo,
            codigo,
        } => d.pulsar_tecla(dispositivo, *codigo),
        Evento::Serie { dispositivo, bytes } => d.recibir_serie(dispositivo, bytes),
        Evento::Interrupcion { linea } => d.levantar_interrupcion(*linea),
    }
    .map_err(|e| e.to_string())
}

fn correr(d: &mut Depurador, prueba: &Prueba) -> Result<Corrida, String> {
    let limite = prueba.limite();
    let mut siguiente = 0;
    let mut corrida = Corrida {
        error: None,
        limite_alcanzado: false,
    };
    loop {
        let (hechas, ciclos) = {
            let e = d.emulador().estado();
            (e.instrucciones(), e.ciclos())
        };
        while siguiente < prueba.eventos.len() {
            let (momento, evento) = &prueba.eventos[siguiente];
            let toca = match momento {
                Momento::Instruccion(n) => hechas >= *n,
                Momento::Ciclo(c) => ciclos >= *c,
            };
            if !toca {
                break;
            }
            aplicar(d, evento)?;
            siguiente += 1;
        }
        if d.emulador().estado().detenida() {
            return Ok(corrida);
        }
        if hechas >= limite {
            corrida.limite_alcanzado = true;
            return Ok(corrida);
        }
        match d.paso(1) {
            Parada::Fallo {
                error, direccion, ..
            } => {
                corrida.error = Some((error, direccion));
                return Ok(corrida);
            }
            Parada::Detenida => return Ok(corrida),
            _ => {}
        }
    }
}

fn hex(v: u64, ancho: usize) -> String {
    format!("{v:#0w$x}", w = ancho + 2)
}

fn palabras(vs: &[u64], ancho: usize) -> String {
    let t: Vec<String> = vs.iter().map(|v| format!("{v:0ancho$x}")).collect();
    format!("[{}]", t.join(" "))
}

/// Compara lo que la prueba espera con lo que paso. No localiza nada: eso se hace despues.
fn comprobar(d: &mut Depurador, prueba: &Prueba, corrida: &Corrida) -> Vec<Fallo> {
    let m = d.maquina().clone();
    let w = (m.bits_palabra as usize).div_ceil(4);
    let dir_w = (m.bits_direccion as usize).div_ceil(4);
    let mut fallos = Vec::new();
    let nuevo = |que: String, esperado: String, obtenido: String| Fallo {
        que,
        esperado,
        obtenido,
        donde: None,
    };
    let esp = &prueba.def.espera;

    if let Some((e, _)) = &corrida.error {
        // Un fallo de ejecucion hace inutil el resto de las comparaciones.
        fallos.push(nuevo(
            "ejecucion".into(),
            "terminar sin errores".into(),
            e.to_string(),
        ));
        return fallos;
    }
    let esperada = esp.detenida.unwrap_or(true);
    let detenida = d.emulador().estado().detenida();
    if esperada != detenida {
        fallos.push(nuevo(
            "detenida".into(),
            esperada.to_string(),
            if corrida.limite_alcanzado {
                format!(
                    "false (supero el limite de {} instrucciones)",
                    prueba.limite()
                )
            } else {
                detenida.to_string()
            },
        ));
    } else if corrida.limite_alcanzado && esperada {
        fallos.push(nuevo(
            "limite".into(),
            format!("detenerse en {} instrucciones o menos", prueba.limite()),
            "la maquina siguio ejecutando".into(),
        ));
    }
    if let Some(pc) = esp.pc {
        let real = d.emulador().estado().pc();
        if real != pc.0 {
            fallos.push(nuevo("pc".into(), hex(pc.0, dir_w), hex(real, dir_w)));
        }
    }
    for (nombre, v) in &esp.registros {
        let real = d.emulador().estado().registro(nombre).unwrap_or(0);
        let esperado = v.0
            & d.emulador()
                .estado()
                .mascara_de_registro(nombre)
                .unwrap_or(u64::MAX);
        if real != esperado {
            fallos.push(nuevo(
                format!("registro {nombre}"),
                format!("{} ({})", hex(esperado, w), esperado),
                format!("{} ({})", hex(real, w), real),
            ));
        }
    }
    for (nombre, v) in &esp.banderas {
        let real = d.emulador().estado().bandera(nombre).unwrap_or(false);
        if real != *v {
            fallos.push(nuevo(
                format!("bandera {nombre}"),
                u8::from(*v).to_string(),
                u8::from(real).to_string(),
            ));
        }
    }
    for t in &esp.memoria {
        let esperado: Vec<u64> = t
            .palabras
            .iter()
            .map(|p| p.0 & m.palabra_maxima())
            .collect();
        match d.emulador().memoria().volcar(t.direccion.0, esperado.len()) {
            Ok(real) if real == esperado => {}
            Ok(real) => {
                let primera = real
                    .iter()
                    .zip(&esperado)
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                fallos.push(nuevo(
                    format!(
                        "memoria {}..{} (difiere en {})",
                        hex(t.direccion.0, dir_w),
                        hex(t.direccion.0 + esperado.len() as u64 - 1, dir_w),
                        hex(t.direccion.0 + primera as u64, dir_w)
                    ),
                    palabras(&esperado, w),
                    palabras(&real, w),
                ));
            }
            Err(e) => fallos.push(nuevo(
                format!("memoria {}", hex(t.direccion.0, dir_w)),
                palabras(&esperado, w),
                e.to_string(),
            )),
        }
    }
    for s in &esp.salida {
        let esperado: Vec<u8> = match (&s.texto, &s.codigos) {
            (Some(t), _) => t.bytes().collect(),
            (_, Some(c)) => c.iter().map(|x| (x.0 & 0xFF) as u8).collect(),
            _ => Vec::new(),
        };
        let datos = m
            .dispositivos
            .iter()
            .find(|x| x.nombre == s.dispositivo && matches!(x.tipo, TipoDispositivo::Uart))
            .and_then(|x| x.puertos.get("datos").copied());
        let real: Vec<u8> = datos
            .and_then(|p| {
                d.emulador_mut()
                    .puertos_mut()
                    .salida(p)
                    .ok()
                    .map(|v| v.to_vec())
            })
            .unwrap_or_default()
            .iter()
            .map(|v| (*v & 0xFF) as u8)
            .collect();
        if real != esperado {
            let mostrar = |b: &[u8]| {
                if b.iter().all(|c| (0x20..0x7f).contains(c) || *c == b'\n') {
                    format!("{:?}", String::from_utf8_lossy(b))
                } else {
                    palabras(&b.iter().map(|c| u64::from(*c)).collect::<Vec<_>>(), 2)
                }
            };
            fallos.push(nuevo(
                format!("salida de {}", s.dispositivo),
                mostrar(&esperado),
                mostrar(&real),
            ));
        }
    }
    for p in &esp.puertos {
        let esperado: Vec<u64> = p.valores.iter().map(|v| v.0 & m.palabra_maxima()).collect();
        let real = d
            .emulador_mut()
            .puertos_mut()
            .salida(p.puerto.0)
            .map(|v| v.to_vec());
        match real {
            Ok(real) if real == esperado => {}
            Ok(real) => fallos.push(nuevo(
                format!("puerto {}", hex(p.puerto.0, 2)),
                palabras(&esperado, w),
                palabras(&real, w),
            )),
            Err(e) => fallos.push(nuevo(
                format!("puerto {}", hex(p.puerto.0, 2)),
                palabras(&esperado, w),
                e.to_string(),
            )),
        }
    }
    if let Some(c) = &esp.ciclos {
        let real = d.emulador().estado().ciclos();
        if real.abs_diff(c.valor) > c.tolerancia {
            fallos.push(nuevo(
                "ciclos".into(),
                if c.tolerancia == 0 {
                    c.valor.to_string()
                } else {
                    format!("{} (+/- {})", c.valor, c.tolerancia)
                },
                real.to_string(),
            ));
        }
    }
    fallos
}

fn donde(d: &Depurador, instante: u64, pc: u64) -> Donde {
    Donde {
        instante,
        direccion: pc,
        instruccion: vistas::instruccion_en(d, pc).unwrap_or_else(|| "?".to_string()),
        fuente: d.ubicacion_de(pc).map(|(a, l)| (a.to_string(), l)),
    }
}

/// Rellena `donde` de cada fallo con la ultima instruccion que toco lo comprobado; si nadie lo
/// toco, con la ultima instruccion ejecutada.
fn localizar(d: &mut Depurador, prueba: &Prueba, corrida: &Corrida, fallos: &mut [Fallo]) {
    let ultimo = d
        .traza()
        .and_then(|t| t.pasos().last().map(|p| (p.instante, p.pc)));
    for f in fallos.iter_mut() {
        let mut paso = None;
        if let Some(r) = f
            .que
            .strip_prefix("registro ")
            .or_else(|| f.que.strip_prefix("bandera "))
        {
            paso = d.quien_cambio(r).ok().flatten();
        } else if f.que.starts_with("memoria ") {
            // El texto lleva "(difiere en 0x..)": de ahi sale la direccion.
            let dir = f
                .que
                .split("difiere en ")
                .nth(1)
                .and_then(|r| r.trim_end_matches(')').trim().strip_prefix("0x"))
                .and_then(|h| u64::from_str_radix(h, 16).ok());
            if let Some(dir) = dir {
                paso = d.quien_escribio(dir).ok().flatten();
            }
        }
        f.donde = match (paso, &corrida.error, ultimo) {
            (Some(p), _, _) => Some(donde(d, p.instante, p.pc)),
            (None, Some((_, pc)), _) if f.que == "ejecucion" => {
                let n = d.traza().map_or(0, |t| t.longitud());
                Some(donde(d, n, *pc))
            }
            (None, _, Some((i, pc))) => Some(donde(d, i, pc)),
            _ => None,
        };
    }
    let _ = prueba;
}

/// Ejecuta una prueba. Si falla, la ejecuta otra vez con traza para decir donde divergio y
/// devuelve el depurador en el estado exacto en que termino (RF-TST-014).
pub fn ejecutar_con_depurador(
    proyecto: &Proyecto,
    prueba: &Prueba,
) -> (Resultado, Option<Depurador>) {
    let inicio = Instant::now();
    let base = |veredicto, motivo, fallos, ins, ciclos| Resultado {
        nombre: prueba.nombre().to_string(),
        etiquetas: prueba.def.etiquetas.clone(),
        veredicto,
        fallos,
        motivo,
        instrucciones: ins,
        ciclos,
        milisegundos: inicio.elapsed().as_millis() as u64,
    };
    let mut d = match preparar(proyecto, prueba, false) {
        Ok(d) => d,
        Err(e) => return (base(Veredicto::Error, Some(e), vec![], 0, 0), None),
    };
    let corrida = match correr(&mut d, prueba) {
        Ok(c) => c,
        Err(e) => return (base(Veredicto::Error, Some(e), vec![], 0, 0), None),
    };
    let ins = d.emulador().estado().instrucciones();
    let ciclos = d.emulador().estado().ciclos();
    let mut fallos = comprobar(&mut d, prueba, &corrida);
    if fallos.is_empty() {
        return (base(Veredicto::Pasa, None, fallos, ins, ciclos), None);
    }
    // Segunda pasada, con traza, solo para localizar.
    let depurador = match preparar(proyecto, prueba, true) {
        Ok(mut d2) => match correr(&mut d2, prueba) {
            Ok(c2) => {
                localizar(&mut d2, prueba, &c2, &mut fallos);
                Some(d2)
            }
            Err(_) => None,
        },
        Err(_) => None,
    };
    (base(Veredicto::Falla, None, fallos, ins, ciclos), depurador)
}

pub fn ejecutar(proyecto: &Proyecto, prueba: &Prueba) -> Resultado {
    ejecutar_con_depurador(proyecto, prueba).0
}
