//! El nucleo del depurador: ejecucion controlada, puntos de ruptura y de observacion.
//!
//! No imprime nada: cada operacion devuelve datos y [`crate::vistas`] los convierte en texto.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Maquina, SentidoPila};
use lorenz_emu::{AccesoVigilado, Emulador, ErrorEjecucion, TipoAcceso};
use lorenz_ens::ensamblador::Objeto;
use lorenz_ens::expr::Entorno;

use crate::condicion::Condicion;
use crate::error::ErrorDepurador;

/// Que accesos a una celda detienen la ejecucion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vigilar {
    Lectura,
    Escritura,
    Acceso,
}

impl Vigilar {
    pub fn nombre(self) -> &'static str {
        match self {
            Vigilar::Lectura => "lectura",
            Vigilar::Escritura => "escritura",
            Vigilar::Acceso => "acceso",
        }
    }

    fn cubre(self, tipo: TipoAcceso) -> bool {
        matches!(
            (self, tipo),
            (Vigilar::Acceso, _)
                | (Vigilar::Lectura, TipoAcceso::Lectura)
                | (Vigilar::Escritura, TipoAcceso::Escritura)
        )
    }
}

/// Un punto de ruptura por direccion (RF-DEP-011), con condicion opcional (RF-DEP-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ruptura {
    pub id: u32,
    pub direccion: u64,
    pub condicion: Option<Condicion>,
    pub activa: bool,
    /// Cuantas veces detuvo la ejecucion.
    pub aciertos: u64,
}

/// Un punto de observacion sobre una celda (RF-DEP-014, RF-DEP-015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observacion {
    pub id: u32,
    pub direccion: u64,
    pub tipo: Vigilar,
    pub aciertos: u64,
}

/// Por que se detuvo la ejecucion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parada {
    /// Se ejecutaron las instrucciones pedidas.
    Paso {
        instrucciones: u64,
    },
    /// La maquina se detuvo.
    Detenida,
    Ruptura {
        id: u32,
        direccion: u64,
    },
    Observacion {
        id: u32,
        acceso: AccesoVigilado,
        /// Valor de la celda despues del acceso.
        valor: Option<u64>,
    },
    /// Se llego a la direccion pedida con `hasta`.
    Llegada {
        direccion: u64,
    },
    /// Se volvio de la subrutina actual.
    Retorno,
    /// Se agoto el limite de instrucciones.
    Limite {
        instrucciones: u64,
    },
    /// Un acceso invalido u otro fallo: indica que instruccion lo causo (RF-DEP-016).
    Fallo {
        error: ErrorEjecucion,
        direccion: u64,
        instruccion: Option<String>,
    },
    /// La condicion de un punto de ruptura no se pudo evaluar.
    CondicionInvalida {
        id: u32,
        motivo: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum Modo {
    Pasos(u64),
    Continuar,
    Hasta(u64),
    Retorno { sp: u64 },
}

/// Un emulador bajo control del usuario.
pub struct Depurador {
    pub(crate) emu: Emulador,
    pub(crate) maquina: Maquina,
    pub(crate) simbolos: BTreeMap<String, i128>,
    pub(crate) etiquetas: BTreeMap<u64, String>,
    pub(crate) ubicaciones: BTreeMap<u64, (String, usize)>,
    pub(crate) fuentes: BTreeMap<String, Vec<String>>,
    rupturas: Vec<Ruptura>,
    observaciones: Vec<Observacion>,
    siguiente_id: u32,
}

/// Lo que las condiciones ven de la maquina.
struct Vista<'a> {
    emu: &'a Emulador,
    simbolos: &'a BTreeMap<String, i128>,
}

impl Entorno for Vista<'_> {
    fn simbolo(&self, nombre: &str) -> Option<i128> {
        let estado = self.emu.estado();
        if let Ok(v) = estado.registro(nombre) {
            return Some(v as i128);
        }
        if let Ok(b) = estado.bandera(nombre) {
            return Some(i128::from(b));
        }
        match nombre.to_ascii_lowercase().as_str() {
            "pc" => return Some(estado.pc() as i128),
            "sp" => return estado.sp().map(|v| v as i128),
            "ciclos" => return Some(estado.ciclos() as i128),
            "instrucciones" => return Some(estado.instrucciones() as i128),
            _ => {}
        }
        self.simbolos.get(nombre).copied()
    }

    fn aqui(&self) -> i128 {
        self.emu.estado().pc() as i128
    }

    fn indice(&self, nombre: &str, indice: i128) -> Option<i128> {
        if nombre.eq_ignore_ascii_case("mem") && indice >= 0 {
            return self
                .emu
                .memoria()
                .leer_privilegiado(indice as u64)
                .ok()
                .map(i128::from);
        }
        None
    }
}

impl Depurador {
    pub fn nuevo(emulador: Emulador) -> Self {
        let maquina = emulador.maquina().clone();
        Depurador {
            emu: emulador,
            maquina,
            simbolos: BTreeMap::new(),
            etiquetas: BTreeMap::new(),
            ubicaciones: BTreeMap::new(),
            fuentes: BTreeMap::new(),
            rupturas: Vec::new(),
            observaciones: Vec::new(),
            siguiente_id: 1,
        }
    }

    pub fn emulador(&self) -> &Emulador {
        &self.emu
    }

    pub fn emulador_mut(&mut self) -> &mut Emulador {
        &mut self.emu
    }

    pub fn maquina(&self) -> &Maquina {
        &self.maquina
    }

    /// Incorpora lo que el ensamblador sabe del programa: simbolos, lineas de codigo fuente y de
    /// que linea salio cada tramo de memoria.
    pub fn con_fuente(&mut self, objeto: &Objeto, fuentes: &[(String, String)]) {
        for (nombre, s) in &objeto.simbolos {
            self.simbolos.insert(nombre.clone(), s.valor);
            if s.etiqueta && s.valor >= 0 {
                self.etiquetas
                    .entry(s.valor as u64)
                    .or_insert_with(|| nombre.clone());
            }
        }
        for e in &objeto.mapa {
            self.ubicaciones
                .entry(e.direccion)
                .or_insert_with(|| (e.archivo.clone(), e.linea));
        }
        for (nombre, texto) in fuentes {
            self.fuentes
                .insert(nombre.clone(), texto.lines().map(str::to_string).collect());
        }
    }

    /// Incorpora etiquetas sueltas (por ejemplo leidas de una tabla de simbolos).
    pub fn con_etiquetas(&mut self, etiquetas: BTreeMap<u64, String>) {
        for (d, n) in etiquetas {
            self.simbolos.insert(n.clone(), d as i128);
            self.etiquetas.entry(d).or_insert(n);
        }
    }

    // --- ubicaciones ---------------------------------------------------------------------

    /// Convierte `0x10`, `42`, una etiqueta o `archivo:linea` en una direccion (RF-DEP-012).
    pub fn resolver(&self, texto: &str) -> Result<u64, ErrorDepurador> {
        let t = texto.trim();
        if let Ok(n) = crate::numero::analizar(t) {
            return Ok(n);
        }
        if let Some(v) = self.simbolos.get(t) {
            return u64::try_from(*v)
                .map_err(|_| ErrorDepurador::UbicacionDesconocida(t.to_string()));
        }
        if let Some((archivo, linea)) = t.rsplit_once(':') {
            if let Ok(linea) = linea.trim().parse::<usize>() {
                return self.direccion_de_linea(archivo.trim(), linea);
            }
        }
        Err(ErrorDepurador::UbicacionDesconocida(t.to_string()))
    }

    /// Direccion de la primera instruccion de una linea de fuente. Si la linea no genera codigo se
    /// usa la siguiente que si lo haga, como hacen los depuradores habituales.
    fn direccion_de_linea(&self, archivo: &str, linea: usize) -> Result<u64, ErrorDepurador> {
        let mut candidatos: Vec<(&str, usize, u64)> = self
            .ubicaciones
            .iter()
            .filter(|(_, (a, l))| {
                *l >= linea
                    && (archivo.is_empty()
                        || a == archivo
                        || a.ends_with(&format!("/{archivo}"))
                        || a.ends_with(archivo))
            })
            .map(|(d, (a, l))| (a.as_str(), *l, *d))
            .collect();
        candidatos.sort_by_key(|(_, l, d)| (*l, *d));
        candidatos
            .first()
            .map(|(_, _, d)| *d)
            .ok_or_else(|| ErrorDepurador::LineaSinCodigo {
                archivo: archivo.to_string(),
                linea,
            })
    }

    /// Nombre de la etiqueta que cae en `direccion`, si hay.
    pub fn etiqueta_en(&self, direccion: u64) -> Option<&str> {
        self.etiquetas.get(&direccion).map(String::as_str)
    }

    /// Archivo y linea de la instruccion en `direccion`, si el programa se ensamblo con fuente.
    pub fn ubicacion_de(&self, direccion: u64) -> Option<(&str, usize)> {
        self.ubicaciones
            .get(&direccion)
            .map(|(a, l)| (a.as_str(), *l))
    }

    // --- puntos de ruptura y observacion -----------------------------------------------

    fn nuevo_id(&mut self) -> u32 {
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        id
    }

    /// Pone un punto de ruptura en `direccion`, con una condicion opcional.
    pub fn poner_ruptura(
        &mut self,
        direccion: u64,
        condicion: Option<&str>,
    ) -> Result<u32, ErrorDepurador> {
        let condicion = condicion.map(Condicion::analizar).transpose()?;
        let id = self.nuevo_id();
        self.rupturas.push(Ruptura {
            id,
            direccion,
            condicion,
            activa: true,
            aciertos: 0,
        });
        Ok(id)
    }

    /// Detiene la ejecucion cuando el programa accede a `direccion` de la forma indicada.
    pub fn observar(&mut self, direccion: u64, tipo: Vigilar) -> Result<u32, ErrorDepurador> {
        if !self.emu.memoria().es_valida(direccion) {
            return Err(ErrorEjecucion::FueraDeRegion { direccion }.into());
        }
        let id = self.nuevo_id();
        let memoria = self.emu.memoria_mut();
        if matches!(tipo, Vigilar::Lectura | Vigilar::Acceso) {
            memoria.vigilar(direccion, TipoAcceso::Lectura);
        }
        if matches!(tipo, Vigilar::Escritura | Vigilar::Acceso) {
            memoria.vigilar(direccion, TipoAcceso::Escritura);
        }
        self.observaciones.push(Observacion {
            id,
            direccion,
            tipo,
            aciertos: 0,
        });
        Ok(id)
    }

    pub fn rupturas(&self) -> &[Ruptura] {
        &self.rupturas
    }

    pub fn observaciones(&self) -> &[Observacion] {
        &self.observaciones
    }

    /// Quita un punto de ruptura o de observacion por su numero.
    pub fn borrar(&mut self, id: u32) -> Result<(), ErrorDepurador> {
        if let Some(i) = self.rupturas.iter().position(|r| r.id == id) {
            self.rupturas.remove(i);
            return Ok(());
        }
        if let Some(i) = self.observaciones.iter().position(|o| o.id == id) {
            let o = self.observaciones.remove(i);
            self.recalcular_vigilancia(o.direccion);
            return Ok(());
        }
        Err(ErrorDepurador::PuntoInexistente(id))
    }

    /// Activa o desactiva un punto de ruptura sin perder su condicion.
    pub fn activar(&mut self, id: u32, activa: bool) -> Result<(), ErrorDepurador> {
        match self.rupturas.iter_mut().find(|r| r.id == id) {
            Some(r) => {
                r.activa = activa;
                Ok(())
            }
            None => Err(ErrorDepurador::PuntoInexistente(id)),
        }
    }

    /// Tras quitar una observacion, deja vigilada la celda solo si otra la sigue pidiendo.
    fn recalcular_vigilancia(&mut self, direccion: u64) {
        let lectura = self
            .observaciones
            .iter()
            .any(|o| o.direccion == direccion && o.tipo.cubre(TipoAcceso::Lectura));
        let escritura = self
            .observaciones
            .iter()
            .any(|o| o.direccion == direccion && o.tipo.cubre(TipoAcceso::Escritura));
        let m = self.emu.memoria_mut();
        if !lectura {
            m.dejar_de_vigilar(direccion, TipoAcceso::Lectura);
        }
        if !escritura {
            m.dejar_de_vigilar(direccion, TipoAcceso::Escritura);
        }
    }

    // --- ejecucion ------------------------------------------------------------------------

    /// Ejecuta `n` instrucciones, salvo que algo detenga antes la ejecucion (RF-DEP-002).
    pub fn paso(&mut self, n: u64) -> Parada {
        self.correr(Modo::Pasos(n.max(1)), u64::MAX)
    }

    /// Ejecuta hasta un punto de ruptura, una observacion, la parada de la maquina o `limite`
    /// instrucciones.
    pub fn continuar(&mut self, limite: u64) -> Parada {
        self.correr(Modo::Continuar, limite)
    }

    /// Ejecuta hasta llegar a `direccion` (RF-DEP-004).
    pub fn hasta(&mut self, direccion: u64, limite: u64) -> Parada {
        self.correr(Modo::Hasta(direccion), limite)
    }

    /// Ejecuta hasta volver de la subrutina actual (RF-DEP-003): hasta que el puntero de pila
    /// quede por encima del que tenia al empezar.
    pub fn salir(&mut self, limite: u64) -> Result<Parada, ErrorDepurador> {
        let sp = self.emu.estado().sp().ok_or(ErrorDepurador::SinPila)?;
        Ok(self.correr(Modo::Retorno { sp }, limite))
    }

    fn instruccion_en(&self, direccion: u64) -> Option<String> {
        crate::vistas::instruccion_en(self, direccion)
    }

    fn correr(&mut self, modo: Modo, limite: u64) -> Parada {
        let mut hechas = 0u64;
        loop {
            if self.emu.estado().detenida() {
                return Parada::Detenida;
            }
            if hechas >= limite {
                return Parada::Limite {
                    instrucciones: hechas,
                };
            }
            let pc = self.emu.estado().pc();
            let paso = match self.emu.paso() {
                Ok(Some(p)) => p,
                Ok(None) => return Parada::Detenida,
                Err(error) => {
                    let instruccion = self.instruccion_en(pc);
                    return Parada::Fallo {
                        error,
                        direccion: pc,
                        instruccion,
                    };
                }
            };
            hechas += 1;
            if let Some(p) = self.observacion_activada() {
                return p;
            }
            if paso.detenida {
                return Parada::Detenida;
            }
            let nuevo_pc = self.emu.estado().pc();
            if let Some(p) = self.ruptura_activada(nuevo_pc) {
                return p;
            }
            match modo {
                Modo::Pasos(n) if hechas >= n => {
                    return Parada::Paso {
                        instrucciones: hechas,
                    }
                }
                Modo::Hasta(d) if nuevo_pc == d => return Parada::Llegada { direccion: d },
                Modo::Retorno { sp } => {
                    let actual = self.emu.estado().sp().unwrap_or(sp);
                    let salio = match self.maquina.pila {
                        SentidoPila::Descendente => actual > sp,
                        SentidoPila::Ascendente => actual < sp,
                    };
                    if salio {
                        return Parada::Retorno;
                    }
                }
                _ => {}
            }
        }
    }

    fn observacion_activada(&mut self) -> Option<Parada> {
        let accesos = self.emu.memoria_mut().tomar_accesos();
        for a in accesos {
            let tipo = a.tipo;
            if let Some(o) = self
                .observaciones
                .iter_mut()
                .find(|o| o.direccion == a.direccion && o.tipo.cubre(tipo))
            {
                o.aciertos += 1;
                let id = o.id;
                let valor = self.emu.memoria().leer_privilegiado(a.direccion).ok();
                return Some(Parada::Observacion {
                    id,
                    acceso: a,
                    valor,
                });
            }
        }
        None
    }

    fn ruptura_activada(&mut self, pc: u64) -> Option<Parada> {
        for i in 0..self.rupturas.len() {
            let r = &self.rupturas[i];
            if !r.activa || r.direccion != pc {
                continue;
            }
            let id = r.id;
            if let Some(c) = &r.condicion {
                let vista = Vista {
                    emu: &self.emu,
                    simbolos: &self.simbolos,
                };
                match c.evaluar(&vista) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        return Some(Parada::CondicionInvalida {
                            id,
                            motivo: e.to_string(),
                        })
                    }
                }
            }
            self.rupturas[i].aciertos += 1;
            return Some(Parada::Ruptura { id, direccion: pc });
        }
        None
    }

    /// Evalua una expresion sobre el estado actual: registros, banderas, simbolos y `mem[...]`.
    pub fn evaluar(&self, texto: &str) -> Result<i128, ErrorDepurador> {
        let e = lorenz_ens::expr::Expr::analizar(texto)
            .map_err(|e| ErrorDepurador::Condicion(e.to_string()))?;
        e.evaluar(&Vista {
            emu: &self.emu,
            simbolos: &self.simbolos,
        })
        .map_err(|e| ErrorDepurador::Condicion(e.to_string()))
    }
}
