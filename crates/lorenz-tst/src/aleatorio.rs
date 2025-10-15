//! Programas aleatorios, comparacion entre dos implementaciones y reduccion de fallos
//! (RF-TST-024, RF-TST-025, RF-TST-026, RF-TST-035).
//!
//! Los programas son lineales: solo usan instrucciones que no saltan, no se detienen y no tocan
//! puertos, y terminan con una instruccion que detiene la maquina. Siempre son validos porque se
//! arman con el codificador de la propia arquitectura. Todo sale de una semilla, de modo que un
//! fallo se repite con la misma semilla.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Instruccion, Maquina, Operando, RolRegistro, TipoOperando};
use lorenz_arch::semantica::ast::{Destino, Sentencia};
use lorenz_emu::{comparar, Ejecutor, Emulador, Veredicto};
use lorenz_ens::codificar::codificar;
use lorenz_ens::desensamblador::{Desensamblador, Linea};
use serde::{Deserialize, Serialize};

use crate::casos::{candidatos, region_de_pruebas, valores_de_codificacion};
use crate::ejecutor_referencia::EjecutorReferencia;
use crate::referencia::usa_puertos;

pub const FORMATO_INFORME: &str = "lorenz-aleatorio-0.1";
pub const SALIDA_OK: i32 = 0;
pub const SALIDA_DIVERGEN: i32 = 1;

/// Generador de numeros pseudoaleatorios (SplitMix64): pequeno, rapido y reproducible.
#[derive(Debug, Clone)]
pub struct Aleatorio {
    estado: u64,
}

impl Aleatorio {
    pub fn nuevo(semilla: u64) -> Self {
        Aleatorio { estado: semilla }
    }

    pub fn siguiente(&mut self) -> u64 {
        self.estado = self.estado.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.estado;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un numero en `0..n` (`n` mayor que cero).
    pub fn por_debajo_de(&mut self, n: u64) -> u64 {
        self.siguiente() % n.max(1)
    }

    /// Un numero entre `minimo` y `maximo`, ambos incluidos.
    pub fn entre(&mut self, minimo: i128, maximo: i128) -> i128 {
        let ancho = (maximo - minimo + 1).max(1) as u128;
        minimo
            + ((u128::from(self.siguiente()) << 64 | u128::from(self.siguiente())) % ancho) as i128
    }
}

#[derive(Debug, Clone)]
pub struct Opciones {
    pub semilla: u64,
    /// Cuantos programas se generan.
    pub programas: usize,
    /// Instrucciones por programa, sin contar la que detiene la maquina.
    pub longitud: usize,
    /// Instrucciones que se dejan correr como maximo en cada programa.
    pub limite: u64,
    /// Reducir los programas que divergen.
    pub reducir: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones {
            semilla: 0,
            programas: 100,
            longitud: 20,
            limite: 10_000,
            reducir: true,
        }
    }
}

/// Un programa aleatorio con el estado de partida de los registros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramaAleatorio {
    /// Direccion donde se carga: la de arranque de la maquina.
    pub origen: u64,
    /// Las palabras de cada instruccion, en orden.
    pub instrucciones: Vec<Vec<u64>>,
    /// La instruccion que detiene la maquina, si la arquitectura tiene una.
    pub fin: Vec<u64>,
    pub registros: Vec<(String, u64)>,
}

impl ProgramaAleatorio {
    pub fn palabras(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.instrucciones.iter().flatten().copied().collect();
        v.extend(&self.fin);
        v
    }

    /// El texto en ensamblador de cada instruccion.
    pub fn a_texto(&self, maquina: &Maquina) -> Vec<String> {
        let Ok(d) = Desensamblador::nuevo(maquina) else {
            return Vec::new();
        };
        d.desensamblar(self.origen, &self.palabras(), None)
            .into_iter()
            .map(|l| match l {
                Linea::Instruccion { texto, .. } => texto,
                Linea::Dato { palabra, .. } => format!(".byte {palabra:#x}"),
            })
            .collect()
    }
}

fn asigna_pc(maquina: &Maquina, ins: &Instruccion) -> bool {
    let pc = maquina
        .registro_con_rol(RolRegistro::ContadorPrograma)
        .map(|r| r.nombre.clone())
        .unwrap_or_default();
    fn recorre(s: &[Sentencia], pc: &str) -> bool {
        s.iter().any(|s| match s {
            Sentencia::Asignar(Destino::Nombre(n), _) => n == "pc" || n == pc,
            Sentencia::Si(_, c) => recorre(c, pc),
            _ => false,
        })
    }
    recorre(&ins.programa.sentencias, &pc)
}

fn detiene(ins: &Instruccion) -> bool {
    ins.programa
        .sentencias
        .iter()
        .any(|s| matches!(s, Sentencia::Detener))
}

/// Instrucciones que un programa lineal puede usar.
fn elegibles(maquina: &Maquina) -> Vec<usize> {
    maquina
        .instrucciones
        .iter()
        .enumerate()
        .filter(|(_, i)| !asigna_pc(maquina, i) && !detiene(i) && !usa_puertos(i))
        .filter(|(_, i)| {
            // Un salto relativo o un puerto no tienen sentido en un programa lineal.
            i.operandos
                .iter()
                .all(|o| !matches!(o.tipo, TipoOperando::Desplazamiento | TipoOperando::Puerto))
        })
        .map(|(n, _)| n)
        .collect()
}

fn valor_de(maquina: &Maquina, ins: &Instruccion, o: &Operando, rng: &mut Aleatorio) -> i128 {
    let lista = candidatos(maquina, ins, o);
    // Tres de cada cuatro veces un valor "interesante" (extremos, cero, uno...).
    if rng.por_debajo_de(4) != 0 {
        return lista[rng.por_debajo_de(lista.len() as u64) as usize];
    }
    match &o.tipo {
        TipoOperando::Registro { permitidos } => rng.entre(0, permitidos.len() as i128 - 1),
        TipoOperando::Inmediato { rango, .. } => {
            rng.entre(i128::from(rango.0), i128::from(rango.1))
        }
        TipoOperando::Direccion => match region_de_pruebas(maquina) {
            Some(r) => rng.entre(i128::from(r.desde), i128::from(r.hasta)),
            None => lista[rng.por_debajo_de(lista.len() as u64) as usize],
        },
        _ => lista[rng.por_debajo_de(lista.len() as u64) as usize],
    }
}

/// Arma un programa con `longitud` instrucciones elegidas al azar.
pub fn generar_programa(
    maquina: &Maquina,
    rng: &mut Aleatorio,
    longitud: usize,
) -> Result<ProgramaAleatorio, String> {
    let pool = elegibles(maquina);
    if pool.is_empty() {
        return Err(
            "la arquitectura no tiene instrucciones que un programa lineal pueda usar".into(),
        );
    }
    let origen = maquina.arranque;
    let mut direccion = origen;
    let mut instrucciones = Vec::with_capacity(longitud);
    for _ in 0..longitud {
        let ins = &maquina.instrucciones[pool[rng.por_debajo_de(pool.len() as u64) as usize]];
        let operandos: Vec<(String, i128)> = ins
            .operandos
            .iter()
            .map(|o| (o.nombre.clone(), valor_de(maquina, ins, o, rng)))
            .collect();
        let valores = valores_de_codificacion(ins, &operandos, direccion);
        let palabras = codificar(maquina, ins, &valores, direccion)
            .map_err(|(n, c)| format!("{}: operando `{n}`: {c:?}", ins.mnemonico))?;
        direccion += palabras.len() as u64;
        instrucciones.push(palabras);
    }
    let fin = match maquina
        .instrucciones
        .iter()
        .find(|i| detiene(i) && i.operandos.is_empty())
    {
        Some(h) => codificar(maquina, h, &BTreeMap::new(), direccion)
            .map_err(|(n, c)| format!("{}: `{n}`: {c:?}", h.mnemonico))?,
        None => Vec::new(),
    };
    // Estado de partida: registros generales y puntero de pila al azar.
    let ram = region_de_pruebas(maquina);
    let registros = maquina
        .registros
        .iter()
        .filter(|r| {
            !matches!(
                r.rol,
                RolRegistro::ContadorPrograma | RolRegistro::Banderas | RolRegistro::Fijo
            )
        })
        .map(|r| {
            let m = if r.bits >= 64 {
                u64::MAX
            } else {
                (1u64 << r.bits) - 1
            };
            let v = match (r.rol, ram) {
                (RolRegistro::PunteroPila, Some(g)) => {
                    g.desde + 16 + rng.por_debajo_de(g.tamano().saturating_sub(32).max(1))
                }
                _ => rng.siguiente(),
            };
            (r.nombre.clone(), v & m)
        })
        .collect();
    Ok(ProgramaAleatorio {
        origen,
        instrucciones,
        fin,
        registros,
    })
}

/// El emulador listo para correr un programa.
pub fn emulador_para(maquina: &Maquina, programa: &ProgramaAleatorio) -> Result<Emulador, String> {
    let mut emu = Emulador::nuevo(maquina.clone()).map_err(|e| e.to_string())?;
    emu.cargar_programa(programa.origen, &programa.palabras())
        .map_err(|e| e.to_string())?;
    for (n, v) in &programa.registros {
        emu.estado_mut()
            .fijar_registro(n, *v)
            .map_err(|e| e.to_string())?;
    }
    Ok(emu)
}

/// La referencia lista para correr un programa.
pub fn referencia_para<'m>(
    maquina: &'m Maquina,
    programa: &ProgramaAleatorio,
) -> Result<EjecutorReferencia<'m>, String> {
    let mut r = EjecutorReferencia::nuevo(maquina)?;
    r.cargar(programa.origen, &programa.palabras())?;
    for (n, v) in &programa.registros {
        r.fijar_registro(n, *v);
    }
    Ok(r)
}

/// Algoritmo de reduccion de casos (*delta debugging*): quita trozos de `items` mientras
/// `sigue_fallando` siga diciendo que el fallo se mantiene (RF-TST-026).
pub fn reducir<T: Clone>(items: Vec<T>, sigue_fallando: &mut dyn FnMut(&[T]) -> bool) -> Vec<T> {
    let mut actual = items;
    let mut partes = 2usize;
    while actual.len() >= 2 {
        let tam = actual.len().div_ceil(partes);
        let mut reducido = false;
        let mut inicio = 0;
        while inicio < actual.len() {
            let fin = (inicio + tam).min(actual.len());
            let mut sin: Vec<T> = actual[..inicio].to_vec();
            sin.extend_from_slice(&actual[fin..]);
            if !sin.is_empty() && sigue_fallando(&sin) {
                actual = sin;
                partes = (partes - 1).max(2);
                reducido = true;
                break;
            }
            inicio = fin;
        }
        if !reducido {
            if partes >= actual.len() {
                break;
            }
            partes = (partes * 2).min(actual.len());
        }
    }
    // Un programa de una sola instruccion todavia puede quedar sin ella.
    if actual.len() == 1 && sigue_fallando(&[]) {
        actual.clear();
    }
    actual
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergencia {
    /// Semilla que regenera este programa: `--semilla` con `--programas 1`.
    pub semilla_del_programa: u64,
    /// Instruccion (la primera es la 0) en que las dos implementaciones se separan.
    pub instruccion: u64,
    pub diferencias: Vec<String>,
    pub programa: Vec<String>,
    pub registros_iniciales: Vec<(String, u64)>,
    /// El programa mas corto que sigue divergiendo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub programa_reducido: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registros_reducidos: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformeAleatorio {
    pub formato: String,
    pub maquina: String,
    pub semilla: u64,
    pub programas: usize,
    /// Programas que no se pudieron comparar por tocar algo que una implementacion no modela.
    pub omitidos: usize,
    pub instrucciones_comparadas: u64,
    pub divergencias: Vec<Divergencia>,
}

impl InformeAleatorio {
    pub fn codigo_de_salida(&self) -> i32 {
        if self.divergencias.is_empty() {
            SALIDA_OK
        } else {
            SALIDA_DIVERGEN
        }
    }

    pub fn a_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("el informe siempre se serializa");
        s.push('\n');
        s
    }

    pub fn desde_json(texto: &str) -> Result<Self, String> {
        let i: InformeAleatorio = serde_json::from_str(texto).map_err(|e| e.to_string())?;
        if i.formato != FORMATO_INFORME {
            return Err(format!("formato desconocido `{}`", i.formato));
        }
        Ok(i)
    }

    pub fn a_texto(&self) -> String {
        use std::fmt::Write;
        let mut t = format!(
            "aleatorio en {} (semilla {}): {} programas, {} instrucciones comparadas",
            self.maquina, self.semilla, self.programas, self.instrucciones_comparadas
        );
        if self.omitidos > 0 {
            let _ = write!(t, ", {} omitidos", self.omitidos);
        }
        let _ = writeln!(t);
        for d in &self.divergencias {
            let _ = writeln!(
                t,
                "DIVERGE (semilla {}) en la instruccion {}:",
                d.semilla_del_programa, d.instruccion
            );
            for x in &d.diferencias {
                let _ = writeln!(t, "    {x}");
            }
            let (prog, regs) = if d.programa_reducido.is_empty() {
                (&d.programa, &d.registros_iniciales)
            } else {
                (&d.programa_reducido, &d.registros_reducidos)
            };
            let _ = writeln!(
                t,
                "  {} ({} instrucciones, {} antes de reducir):",
                if d.programa_reducido.is_empty() {
                    "programa"
                } else {
                    "programa reducido"
                },
                prog.len(),
                d.programa.len()
            );
            for l in prog {
                let _ = writeln!(t, "      {l}");
            }
            let regs: Vec<String> = regs.iter().map(|(n, v)| format!("{n}={v:#x}")).collect();
            let _ = writeln!(t, "  registros iniciales: {}", regs.join(" "));
        }
        let _ = writeln!(
            t,
            "{}",
            if self.divergencias.is_empty() {
                "sin divergencias".to_string()
            } else {
                format!("{} divergencias", self.divergencias.len())
            }
        );
        t
    }
}

/// Como se arma una implementacion para un programa dado.
pub type Fabrica<'a> = dyn Fn(&ProgramaAleatorio) -> Result<Box<dyn Ejecutor + 'a>, String> + 'a;

/// Resultado de comparar un programa en las dos implementaciones.
enum Resultado {
    Igual(u64),
    Omitido,
    Diverge(u64, Vec<String>),
}

fn comparar_programa(
    p: &ProgramaAleatorio,
    primera: &Fabrica<'_>,
    segunda: &Fabrica<'_>,
    limite: u64,
) -> Result<Resultado, String> {
    let mut a = primera(p)?;
    let mut b = segunda(p)?;
    let v = comparar(a.as_mut(), b.as_mut(), limite);
    Ok(match v {
        Veredicto::Divergen {
            instruccion,
            diferencias,
            ..
        } => {
            if a.no_comparable() || b.no_comparable() {
                Resultado::Omitido
            } else {
                Resultado::Diverge(instruccion, diferencias)
            }
        }
        Veredicto::Iguales { instrucciones } => Resultado::Igual(instrucciones),
        Veredicto::LimiteAlcanzado { limite } => Resultado::Igual(limite),
    })
}

/// Reduce un programa que diverge: primero quita instrucciones y despues pone a cero los
/// registros iniciales que no hacen falta.
fn reducir_programa(
    p: &ProgramaAleatorio,
    primera: &Fabrica<'_>,
    segunda: &Fabrica<'_>,
    limite: u64,
) -> ProgramaAleatorio {
    let diverge = |q: &ProgramaAleatorio| {
        matches!(
            comparar_programa(q, primera, segunda, limite),
            Ok(Resultado::Diverge(..))
        )
    };
    let base = p.clone();
    let instrucciones = reducir(p.instrucciones.clone(), &mut |trozo: &[Vec<u64>]| {
        // Quitar instrucciones desplaza las direcciones, pero el programa es lineal.
        let q = ProgramaAleatorio {
            instrucciones: trozo.to_vec(),
            ..base.clone()
        };
        diverge(&q)
    });
    let mut reducido = ProgramaAleatorio {
        instrucciones,
        ..p.clone()
    };
    for i in 0..reducido.registros.len() {
        let antes = reducido.registros[i].1;
        if antes == 0 {
            continue;
        }
        reducido.registros[i].1 = 0;
        if !diverge(&reducido) {
            reducido.registros[i].1 = antes;
        }
    }
    reducido
}

/// Genera programas y compara la ejecucion de dos implementaciones cualesquiera.
pub fn ejecutar_aleatorio_con(
    maquina: &Maquina,
    opciones: &Opciones,
    primera: &Fabrica<'_>,
    segunda: &Fabrica<'_>,
) -> Result<InformeAleatorio, String> {
    let mut divergencias = Vec::new();
    let (mut omitidos, mut comparadas) = (0usize, 0u64);
    for k in 0..opciones.programas {
        let semilla_del_programa = opciones.semilla.wrapping_add(k as u64);
        let mut rng = Aleatorio::nuevo(semilla_del_programa);
        let p = generar_programa(maquina, &mut rng, opciones.longitud)?;
        match comparar_programa(&p, primera, segunda, opciones.limite)? {
            Resultado::Igual(n) => comparadas += n,
            Resultado::Omitido => omitidos += 1,
            Resultado::Diverge(instruccion, diferencias) => {
                let mut d = Divergencia {
                    semilla_del_programa,
                    instruccion,
                    diferencias,
                    programa: p.a_texto(maquina),
                    registros_iniciales: p.registros.clone(),
                    programa_reducido: Vec::new(),
                    registros_reducidos: Vec::new(),
                };
                if opciones.reducir {
                    let r = reducir_programa(&p, primera, segunda, opciones.limite);
                    d.programa_reducido = r.a_texto(maquina);
                    d.registros_reducidos = r.registros.clone();
                }
                divergencias.push(d);
            }
        }
    }
    Ok(InformeAleatorio {
        formato: FORMATO_INFORME.to_string(),
        maquina: maquina.nombre.clone(),
        semilla: opciones.semilla,
        programas: opciones.programas,
        omitidos,
        instrucciones_comparadas: comparadas,
        divergencias,
    })
}

/// Compara el emulador con el evaluador de referencia.
pub fn ejecutar_aleatorio(
    maquina: &Maquina,
    opciones: &Opciones,
) -> Result<InformeAleatorio, String> {
    ejecutar_aleatorio_con(
        maquina,
        opciones,
        &|p| Ok(Box::new(emulador_para(maquina, p)?)),
        &|p| Ok(Box::new(referencia_para(maquina, p)?)),
    )
}
