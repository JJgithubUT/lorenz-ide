//! El nucleo del depurador: ejecucion controlada, puntos de ruptura y de observacion.
//!
//! No imprime nada: cada operacion devuelve datos y [`crate::vistas`] los convierte en texto.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Maquina, SentidoPila};
use lorenz_emu::{AccesoVigilado, Emulador, ErrorEjecucion, Estado, Paso, TipoAcceso};
use lorenz_ens::ensamblador::Objeto;
use lorenz_ens::expr::Entorno;

use crate::condicion::Condicion;
use crate::error::ErrorDepurador;
use crate::traza::{CambioBandera, CambioRegistro, Intervencion, PasoTraza, Traza};

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
    traza: Option<Traza>,
    /// Instante en el que esta la maquina cuando hay traza; si es menor que la longitud de la
    /// traza, se esta mirando el pasado.
    cursor: u64,
    pub(crate) avisos: Vec<String>,
    /// Vigilancia de errores de programa, si esta activa.
    pub(crate) vigilancia: Option<crate::avisos::Vigilancia>,
    /// Las instrucciones ejecutadas desde que se pidio recogerlas; `None` si no se recogen.
    pasos_recogidos: Option<Vec<Paso>>,
    /// Fallos que se inyectan al llegar a su instante (RF-DEP-042).
    inyecciones: Vec<crate::inyeccion::Programada>,
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
            traza: None,
            cursor: 0,
            avisos: Vec::new(),
            vigilancia: None,
            pasos_recogidos: None,
            inyecciones: Vec::new(),
        }
    }

    /// Empieza o deja de recoger cada instruccion ejecutada, para medir cobertura (RF-TST-021).
    pub fn recoger_pasos(&mut self, activa: bool) {
        self.pasos_recogidos = activa.then(Vec::new);
    }

    /// Entrega las instrucciones recogidas desde la ultima vez y vacia la lista.
    pub fn tomar_pasos(&mut self) -> Vec<Paso> {
        self.pasos_recogidos
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
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
        self.escribir_en_el_presente();
        loop {
            if self.emu.estado().detenida() {
                return Parada::Detenida;
            }
            if hechas >= limite {
                return Parada::Limite {
                    instrucciones: hechas,
                };
            }
            self.inyectar_pendientes();
            let pc = self.emu.estado().pc();
            let antes = self
                .traza
                .as_ref()
                .filter(|t| !t.llena())
                .map(|_| self.emu.estado().clone());
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
            if let Some(v) = &mut self.pasos_recogidos {
                v.push(paso.clone());
            }
            if let Some(antes) = antes {
                self.anotar_paso(&antes, &paso);
            }
            self.revisar_avisos(&paso);
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

// --- traza y retroceso -----------------------------------------------------------------------

impl Depurador {
    /// Activa o desactiva la grabacion de la traza (RF-DEP-020).
    ///
    /// Al activarla se empieza una traza nueva desde el estado actual; al desactivarla se descarta.
    pub fn grabar(&mut self, activa: bool) {
        if activa {
            if self.traza.is_none() {
                self.traza = Some(Traza::nueva(self.emu.instantanea()));
                self.cursor = 0;
                self.emu.memoria_mut().registrar_escrituras(true);
            }
        } else {
            self.traza = None;
            self.cursor = 0;
            self.emu.memoria_mut().registrar_escrituras(false);
        }
    }

    pub fn traza(&self) -> Option<&Traza> {
        self.traza.as_ref()
    }

    pub fn traza_mut(&mut self) -> Option<&mut Traza> {
        self.traza.as_mut()
    }

    /// Instante en el que esta la maquina, contado en pasos desde que empezo la grabacion.
    pub fn instante(&self) -> u64 {
        self.cursor
    }

    /// `true` si se esta mirando un instante anterior al final de la traza.
    pub fn en_el_pasado(&self) -> bool {
        self.traza
            .as_ref()
            .is_some_and(|t| self.cursor < t.longitud())
    }

    /// Avisos pendientes de mostrar, como el de la traza llena (RF-DEP-028).
    pub fn tomar_avisos(&mut self) -> Vec<String> {
        std::mem::take(&mut self.avisos)
    }

    /// Si se estaba mirando el pasado y se ejecuta, esa historia se descarta: desde ahi sigue otra.
    fn escribir_en_el_presente(&mut self) {
        if let Some(t) = self.traza.as_mut() {
            if self.cursor < t.longitud() {
                t.truncar(self.cursor);
            }
        }
    }

    fn anotar_paso(&mut self, antes: &Estado, paso: &lorenz_emu::Paso) {
        let despues = self.emu.estado();
        let pc = despues.nombre_del_pc().to_string();
        let registros: Vec<CambioRegistro> = antes
            .nombres()
            .iter()
            .filter(|n| !n.eq_ignore_ascii_case(&pc))
            .filter_map(|n| {
                let a = antes.registro(n).ok()?;
                let d = despues.registro(n).ok()?;
                (a != d).then(|| CambioRegistro {
                    nombre: n.clone(),
                    antes: a,
                    despues: d,
                })
            })
            .collect();
        let banderas: Vec<CambioBandera> = antes
            .nombres_de_banderas()
            .into_iter()
            .filter_map(|n| {
                let a = antes.bandera(n).ok()?;
                let d = despues.bandera(n).ok()?;
                (a != d).then(|| CambioBandera {
                    nombre: n.to_string(),
                    antes: a,
                    despues: d,
                })
            })
            .collect();
        let escrituras = self.emu.memoria_mut().tomar_escrituras();
        let registro = PasoTraza {
            instante: self.cursor,
            pc: paso.direccion,
            mnemonico: paso.mnemonico.clone(),
            ciclos: paso.ciclos,
            registros,
            banderas,
            escrituras,
        };
        let emu = &self.emu;
        let Some(t) = self.traza.as_mut() else { return };
        if t.anotar(registro, || emu.instantanea()) {
            self.cursor += 1;
        }
        if t.llena() {
            self.avisos.push(format!(
                "la traza alcanzo su limite de {} pasos; ya no se graba (usa `traza limite N` o `traza borrar`)",
                t.limite()
            ));
        }
    }

    /// Lleva la maquina al instante `destino` de la traza (RF-DEP-021, RF-DEP-022, RF-DEP-023).
    pub fn ir_a(&mut self, destino: u64) -> Result<(), ErrorDepurador> {
        let t = self.traza.as_ref().ok_or(ErrorDepurador::SinTraza)?;
        if destino > t.longitud() {
            return Err(ErrorDepurador::InstanteInvalido {
                instante: destino,
                maximo: t.longitud(),
            });
        }
        let final_ = t.longitud();
        let (desde, copia) = {
            let (d, c) = t.instantanea_hasta(destino);
            (d, c.clone())
        };
        let intervenciones: Vec<(u64, Intervencion)> = t
            .intervenciones()
            .iter()
            .filter(|(k, _)| *k >= desde && *k <= destino)
            .cloned()
            .collect();
        self.emu.restaurar(&copia)?;
        for k in desde..=destino {
            // Lo hecho a mano en el instante k se aplica antes de ejecutar el paso k; en el
            // instante final tambien, porque ahi esta la maquina viva.
            if k < destino || destino == final_ {
                for (_, i) in intervenciones.iter().filter(|(t, _)| *t == k) {
                    self.aplicar(i)?;
                }
            }
            if k < destino {
                self.emu.paso()?;
                self.emu.memoria_mut().tomar_escrituras();
            }
        }
        self.emu.memoria_mut().tomar_accesos();
        self.restablecer_vigilancia();
        self.cursor = destino;
        Ok(())
    }

    /// Retrocede `n` pasos (RF-DEP-021).
    pub fn atras(&mut self, n: u64) -> Result<(), ErrorDepurador> {
        let destino = self.cursor.saturating_sub(n.max(1));
        self.ir_a(destino)
    }

    /// Avanza `n` pasos por la traza ya grabada, sin ejecutar nada nuevo.
    pub fn adelante(&mut self, n: u64) -> Result<(), ErrorDepurador> {
        let t = self.traza.as_ref().ok_or(ErrorDepurador::SinTraza)?;
        let destino = (self.cursor + n.max(1)).min(t.longitud());
        self.ir_a(destino)
    }

    /// Pone otra maquina en marcha en lugar de la actual, por ejemplo la que sale de reproducir un
    /// fallo. Se pierden la traza y los puntos de observacion; las rupturas se conservan.
    pub fn reemplazar_emulador(&mut self, emulador: Emulador) {
        self.maquina = emulador.maquina().clone();
        self.emu = emulador;
        self.traza = None;
        self.cursor = 0;
        self.observaciones.clear();
    }

    /// Aplica una intervencion guardada, anotandola en la traza como si se hubiera hecho ahora.
    pub fn aplicar_a_mano(&mut self, i: &Intervencion) -> Result<(), ErrorDepurador> {
        match i {
            Intervencion::Registro { nombre, valor } => self.fijar_registro(nombre, *valor),
            Intervencion::Bandera { nombre, encendida } => self.fijar_bandera(nombre, *encendida),
            Intervencion::Memoria { direccion, valor } => self.escribir_memoria(*direccion, *valor),
            Intervencion::Puerto { numero, valor } => self.escribir_puerto(*numero, *valor),
            Intervencion::Entrada { numero, valor } => self.encolar_entrada(*numero, *valor),
            Intervencion::LecturaDePuerto { numero } => self.leer_puerto(*numero).map(|_| ()),
            Intervencion::Tecla {
                dispositivo,
                codigo,
            } => self.pulsar_tecla(dispositivo, *codigo),
            Intervencion::Serie { dispositivo, bytes } => self.recibir_serie(dispositivo, bytes),
            Intervencion::Interrupcion { linea } => self.levantar_interrupcion(*linea),
        }
    }

    fn aplicar(&mut self, i: &Intervencion) -> Result<(), ErrorDepurador> {
        match i {
            Intervencion::Registro { nombre, valor } => self.poner_registro(nombre, *valor),
            Intervencion::Bandera { nombre, encendida } => {
                self.emu.estado_mut().fijar_bandera(nombre, *encendida)?;
            }
            Intervencion::Memoria { direccion, valor } => {
                self.emu
                    .memoria_mut()
                    .escribir_privilegiado(*direccion, *valor)?;
            }
            Intervencion::Puerto { numero, valor } => {
                self.emu.puertos_mut().escribir(*numero, *valor)?;
            }
            Intervencion::Entrada { numero, valor } => {
                self.emu.puertos_mut().encolar_entrada(*numero, *valor)?;
            }
            Intervencion::LecturaDePuerto { numero } => {
                self.emu.puertos_mut().leer(*numero)?;
            }
            Intervencion::Tecla {
                dispositivo,
                codigo,
            } => self.emu.pulsar_tecla(dispositivo, *codigo)?,
            Intervencion::Serie { dispositivo, bytes } => {
                self.emu.recibir_en_uart(dispositivo, bytes)?
            }
            Intervencion::Interrupcion { linea } => self.emu.levantar_interrupcion(*linea)?,
        }
        Ok(())
    }

    fn poner_registro(&mut self, nombre: &str, valor: u64) {
        let e = self.emu.estado_mut();
        if nombre.eq_ignore_ascii_case(e.nombre_del_pc()) {
            e.fijar_pc(valor);
        } else if e
            .nombre_del_sp()
            .is_some_and(|s| s.eq_ignore_ascii_case(nombre))
        {
            e.fijar_sp(valor);
        } else {
            let _ = e.fijar_registro(nombre, valor);
        }
    }

    /// Tras restaurar una instantanea la memoria trae las vigilancias de su momento: se vuelven a
    /// poner las de ahora.
    fn restablecer_vigilancia(&mut self) {
        let m = self.emu.memoria_mut();
        for tipo in [TipoAcceso::Lectura, TipoAcceso::Escritura] {
            for d in m.vigiladas(tipo) {
                m.dejar_de_vigilar(d, tipo);
            }
        }
        for o in &self.observaciones {
            if o.tipo.cubre(TipoAcceso::Lectura) {
                m.vigilar(o.direccion, TipoAcceso::Lectura);
            }
            if o.tipo.cubre(TipoAcceso::Escritura) {
                m.vigilar(o.direccion, TipoAcceso::Escritura);
            }
        }
        m.registrar_escrituras(self.traza.is_some());
    }

    // --- inyeccion de fallos ---------------------------------------------------------------

    /// Programa un fallo para cuando se hayan ejecutado `instante` instrucciones (RF-DEP-042).
    pub fn programar_inyeccion(
        &mut self,
        instante: u64,
        inyeccion: crate::inyeccion::Inyeccion,
    ) -> Result<u32, ErrorDepurador> {
        use crate::inyeccion::Inyeccion as I;
        match &inyeccion {
            I::BitRegistro { nombre, bit } => {
                let r = self
                    .maquina
                    .registros
                    .iter()
                    .find(|r| r.nombre.eq_ignore_ascii_case(nombre))
                    .ok_or_else(|| ErrorDepurador::NombreDesconocido(nombre.clone()))?;
                if *bit >= r.bits {
                    return Err(ErrorDepurador::BitFueraDeRango {
                        bit: *bit,
                        ancho: r.bits,
                    });
                }
            }
            I::BitMemoria { bit, .. } if *bit >= self.maquina.bits_palabra => {
                return Err(ErrorDepurador::BitFueraDeRango {
                    bit: *bit,
                    ancho: self.maquina.bits_palabra,
                });
            }
            _ => {}
        }
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        self.inyecciones.push(crate::inyeccion::Programada {
            id,
            instante,
            inyeccion,
            aplicada: false,
        });
        Ok(id)
    }

    /// Los fallos programados, con su estado.
    pub fn inyecciones(&self) -> &[crate::inyeccion::Programada] {
        &self.inyecciones
    }

    /// Quita un fallo programado.
    pub fn quitar_inyeccion(&mut self, id: u32) -> Result<(), ErrorDepurador> {
        let n = self.inyecciones.len();
        self.inyecciones.retain(|i| i.id != id);
        if self.inyecciones.len() == n {
            return Err(ErrorDepurador::PuntoInexistente(id));
        }
        Ok(())
    }

    /// Aplica los fallos cuyo instante ya llego. Cada resultado queda como aviso.
    fn inyectar_pendientes(&mut self) {
        if self.inyecciones.iter().all(|i| i.aplicada) {
            return;
        }
        let ahora = self.emu.estado().instrucciones();
        for k in 0..self.inyecciones.len() {
            let (aplicada, instante) = (self.inyecciones[k].aplicada, self.inyecciones[k].instante);
            if aplicada || instante > ahora {
                continue;
            }
            self.inyecciones[k].aplicada = true;
            let (id, inyeccion) = (
                self.inyecciones[k].id,
                self.inyecciones[k].inyeccion.clone(),
            );
            let r = self.aplicar_inyeccion(&inyeccion);
            self.avisos.push(match r {
                Ok(()) => format!("fallo {id} inyectado en el instante {ahora}: {inyeccion}"),
                Err(e) => format!("fallo {id} no se pudo inyectar ({inyeccion}): {e}"),
            });
        }
    }

    fn aplicar_inyeccion(&mut self, i: &crate::inyeccion::Inyeccion) -> Result<(), ErrorDepurador> {
        use crate::inyeccion::Inyeccion as I;
        match i {
            I::BitRegistro { nombre, bit } => {
                let actual = self.emu.estado().registro(nombre)?;
                self.fijar_registro(nombre, actual ^ (1u64 << bit))
            }
            I::BitMemoria { direccion, bit } => {
                let actual = self.emu.memoria().leer_privilegiado(*direccion)?;
                self.escribir_memoria(*direccion, actual ^ (1u64 << bit))
            }
            I::PerderEntrada { puerto } => self.leer_puerto(*puerto).map(|_| ()),
            I::Interrupcion { linea } => self.levantar_interrupcion(*linea),
        }
    }

    // --- cambios hechos a mano, anotados en la traza -------------------------------------

    fn anotar_intervencion(&mut self, i: Intervencion) {
        if let Some(t) = self.traza.as_mut() {
            t.intervenir(self.cursor, i);
        }
    }

    /// Cambia un registro (RF-DEP-008). Con grabacion activa queda anotado.
    pub fn fijar_registro(&mut self, nombre: &str, valor: u64) -> Result<(), ErrorDepurador> {
        let e = self.emu.estado();
        let existe = e.registro(nombre).is_ok();
        if !existe {
            return Err(ErrorDepurador::NombreDesconocido(nombre.to_string()));
        }
        self.escribir_en_el_presente_si_hace_falta();
        let e = self.emu.estado_mut();
        if nombre.eq_ignore_ascii_case(e.nombre_del_pc()) {
            e.fijar_pc(valor);
        } else if e
            .nombre_del_sp()
            .is_some_and(|s| s.eq_ignore_ascii_case(nombre))
        {
            e.fijar_sp(valor);
        } else {
            e.fijar_registro(nombre, valor)?;
        }
        self.anotar_intervencion(Intervencion::Registro {
            nombre: nombre.to_string(),
            valor,
        });
        Ok(())
    }

    /// Enciende o apaga una bandera (RF-DEP-008).
    pub fn fijar_bandera(&mut self, nombre: &str, encendida: bool) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.estado_mut().fijar_bandera(nombre, encendida)?;
        self.anotar_intervencion(Intervencion::Bandera {
            nombre: nombre.to_string(),
            encendida,
        });
        Ok(())
    }

    /// Cambia una celda de memoria sin mirar los permisos (RF-DEP-009).
    pub fn escribir_memoria(&mut self, direccion: u64, valor: u64) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu
            .memoria_mut()
            .escribir_privilegiado(direccion, valor)?;
        self.anotar_intervencion(Intervencion::Memoria { direccion, valor });
        Ok(())
    }

    /// Escribe en un puerto (RF-DEP-010).
    pub fn escribir_puerto(&mut self, numero: u64, valor: u64) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.puertos_mut().escribir(numero, valor)?;
        self.anotar_intervencion(Intervencion::Puerto { numero, valor });
        Ok(())
    }

    /// Deja un valor en la cola de entrada de un puerto (RF-DEP-010).
    pub fn encolar_entrada(&mut self, numero: u64, valor: u64) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.puertos_mut().encolar_entrada(numero, valor)?;
        self.anotar_intervencion(Intervencion::Entrada { numero, valor });
        Ok(())
    }

    /// Leer un puerto consume su cola, asi que tambien es un cambio de estado que hay que anotar.
    pub fn leer_puerto(&mut self, numero: u64) -> Result<u64, ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        let v = self.emu.puertos_mut().leer(numero)?;
        self.anotar_intervencion(Intervencion::LecturaDePuerto { numero });
        Ok(v)
    }

    /// Pulsa una tecla en un teclado de la maquina.
    pub fn pulsar_tecla(&mut self, dispositivo: &str, codigo: u64) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.pulsar_tecla(dispositivo, codigo)?;
        self.anotar_intervencion(Intervencion::Tecla {
            dispositivo: dispositivo.to_string(),
            codigo,
        });
        Ok(())
    }

    /// Entrega bytes a una UART como si llegaran por su linea serie.
    pub fn recibir_serie(&mut self, dispositivo: &str, bytes: &[u8]) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.recibir_en_uart(dispositivo, bytes)?;
        self.anotar_intervencion(Intervencion::Serie {
            dispositivo: dispositivo.to_string(),
            bytes: bytes.to_vec(),
        });
        Ok(())
    }

    /// Levanta una linea de interrupcion desde fuera de la maquina.
    pub fn levantar_interrupcion(&mut self, linea: u32) -> Result<(), ErrorDepurador> {
        self.escribir_en_el_presente_si_hace_falta();
        self.emu.levantar_interrupcion(linea)?;
        self.anotar_intervencion(Intervencion::Interrupcion { linea });
        Ok(())
    }

    fn escribir_en_el_presente_si_hace_falta(&mut self) {
        self.escribir_en_el_presente();
    }

    // --- preguntas a la traza ------------------------------------------------------------

    /// El paso que escribio `direccion` por ultima vez antes del instante actual (RF-DEP-024).
    pub fn quien_escribio(&self, direccion: u64) -> Result<Option<PasoTraza>, ErrorDepurador> {
        let t = self.traza.as_ref().ok_or(ErrorDepurador::SinTraza)?;
        Ok(t.ultima_escritura(direccion, self.cursor).cloned())
    }

    /// El paso que cambio un registro o una bandera por ultima vez antes del instante actual
    /// (RF-DEP-025).
    pub fn quien_cambio(&self, nombre: &str) -> Result<Option<PasoTraza>, ErrorDepurador> {
        let t = self.traza.as_ref().ok_or(ErrorDepurador::SinTraza)?;
        let e = self.emu.estado();
        let paso = if e.tiene_bandera(nombre) {
            t.ultimo_cambio_de_bandera(nombre, self.cursor)
        } else if e.registro(nombre).is_ok() {
            t.ultimo_cambio_de_registro(nombre, self.cursor)
        } else {
            return Err(ErrorDepurador::NombreDesconocido(nombre.to_string()));
        };
        Ok(paso.cloned())
    }

    /// El primer instante de la traza en que se cumple la condicion (RF-DEP-026). La maquina
    /// queda donde estaba.
    pub fn buscar(&mut self, condicion: &str) -> Result<Option<u64>, ErrorDepurador> {
        let c = Condicion::analizar(condicion)?;
        let t = self.traza.as_ref().ok_or(ErrorDepurador::SinTraza)?;
        let final_ = t.longitud();
        let origen = self.cursor;
        let copia = t.instantanea_hasta(0).1.clone();
        let intervenciones: Vec<(u64, Intervencion)> = t.intervenciones().to_vec();
        self.emu.restaurar(&copia)?;
        let mut hallado = None;
        let mut fallo = None;
        for k in 0..=final_ {
            if k == final_ {
                for (_, i) in intervenciones.iter().filter(|(t, _)| *t == k) {
                    self.aplicar(i)?;
                }
            }
            let vista = Vista {
                emu: &self.emu,
                simbolos: &self.simbolos,
            };
            match c.evaluar(&vista) {
                Ok(true) => {
                    hallado = Some(k);
                    break;
                }
                Ok(false) => {}
                Err(e) => {
                    fallo = Some(ErrorDepurador::Condicion(e.to_string()));
                    break;
                }
            }
            if k < final_ {
                for (_, i) in intervenciones.iter().filter(|(t, _)| *t == k) {
                    self.aplicar(i)?;
                }
                self.emu.paso()?;
                self.emu.memoria_mut().tomar_escrituras();
            }
        }
        self.ir_a(origen)?;
        match fallo {
            Some(e) => Err(e),
            None => Ok(hallado),
        }
    }
}
