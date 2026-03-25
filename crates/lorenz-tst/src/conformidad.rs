//! Suite de conformidad generada desde la arquitectura (RF-TST-016 a RF-TST-020).
//!
//! Para cada instruccion de la maquina se arman casos con valores limite de sus operandos y
//! estados de partida variados. Cada caso se comprueba de cuatro maneras:
//!
//! * **semantica**: el emulador y el evaluador de referencia (`referencia`) deben dejar el mismo
//!   estado (registros, banderas, memoria, ciclos, salto);
//! * **banderas ajenas**: una bandera que la instruccion no declara en `afecta` ni asigna debe
//!   quedar como estaba;
//! * **ida y vuelta**: las palabras codificadas se desensamblan, el texto se vuelve a ensamblar y
//!   debe dar las mismas palabras, y el decodificador debe recuperar los operandos;
//! * **limites**: un valor justo fuera del rango de un operando debe ser rechazado.

use lorenz_arch::modelo::{Maquina, RolRegistro};
use lorenz_emu::decodificador::Decodificador;
use lorenz_emu::Emulador;
use lorenz_ens::codificar::codificar;
use lorenz_ens::desensamblador::{Desensamblador, Linea};
use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use serde::{Deserialize, Serialize};

use crate::casos::{
    casos_de, es_ram, fuera_de_rango, patrones, region_de_pruebas, valores_de_codificacion, Caso,
};
use crate::referencia::{
    banderas_asignadas, usa_puertos, ErrorRef, EstadoRef, Referencia, ValorDeOperando,
};

pub const FORMATO_INFORME: &str = "lorenz-conformidad-0.1";
pub const SALIDA_OK: i32 = 0;
pub const SALIDA_FALLAN: i32 = 1;

/// Cuantos fallos se detallan por instruccion; el resto solo se cuenta.
const MAX_FALLOS_DETALLADOS: usize = 8;

#[derive(Debug, Clone)]
pub struct Opciones {
    /// Combinaciones de operandos por instruccion como maximo.
    pub max_combinaciones: usize,
    /// Estados de partida distintos que se prueban con cada combinacion.
    pub repeticiones: usize,
    /// Estados de partida distintos que se generan en total.
    pub patrones: usize,
    /// Solo las instrucciones cuyo mnemonico es este (sin distinguir mayusculas).
    pub mnemonico: Option<String>,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones {
            max_combinaciones: 48,
            repeticiones: 3,
            patrones: 8,
            mnemonico: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verificacion {
    Semantica,
    BanderasAjenas,
    IdaYVuelta,
    Limites,
}

impl Verificacion {
    pub fn nombre(self) -> &'static str {
        match self {
            Verificacion::Semantica => "semantica",
            Verificacion::BanderasAjenas => "banderas ajenas",
            Verificacion::IdaYVuelta => "ida y vuelta",
            Verificacion::Limites => "limites",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FalloConformidad {
    pub verificacion: Verificacion,
    pub caso: String,
    pub detalles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultadoInstruccion {
    pub mnemonico: String,
    /// Casos de semantica ejecutados.
    pub casos: usize,
    /// Casos que no se pudieron comprobar (acceso fuera de RAM, puertos...).
    pub omitidos: usize,
    pub ida_y_vuelta: usize,
    pub limites: usize,
    pub total_fallos: usize,
    pub fallos: Vec<FalloConformidad>,
    /// Por que la semantica no se comprobo, si no se hizo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nota: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformeConformidad {
    pub formato: String,
    pub maquina: String,
    pub instrucciones: Vec<ResultadoInstruccion>,
}

impl InformeConformidad {
    pub fn total_fallos(&self) -> usize {
        self.instrucciones.iter().map(|i| i.total_fallos).sum()
    }

    pub fn total_casos(&self) -> usize {
        self.instrucciones
            .iter()
            .map(|i| i.casos + i.ida_y_vuelta + i.limites)
            .sum()
    }

    pub fn codigo_de_salida(&self) -> i32 {
        if self.total_fallos() > 0 {
            SALIDA_FALLAN
        } else {
            SALIDA_OK
        }
    }

    pub fn a_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("el informe siempre se puede serializar")
    }

    pub fn desde_json(texto: &str) -> Result<Self, String> {
        let i: InformeConformidad = serde_json::from_str(texto).map_err(|e| e.to_string())?;
        if i.formato != FORMATO_INFORME {
            return Err(format!("formato desconocido `{}`", i.formato));
        }
        Ok(i)
    }

    pub fn a_texto(&self) -> String {
        let mut t = format!("conformidad de {}\n", self.maquina);
        for i in &self.instrucciones {
            let estado = if i.total_fallos == 0 {
                "ok   "
            } else {
                "FALLA"
            };
            t.push_str(&format!(
                "  {estado} {:<8} {} casos, {} omitidos, {} ida y vuelta, {} limites",
                i.mnemonico, i.casos, i.omitidos, i.ida_y_vuelta, i.limites
            ));
            if let Some(n) = &i.nota {
                t.push_str(&format!(" ({n})"));
            }
            t.push('\n');
            for f in &i.fallos {
                t.push_str(&format!("      [{}] {}\n", f.verificacion.nombre(), f.caso));
                for d in &f.detalles {
                    t.push_str(&format!("          {d}\n"));
                }
            }
            if i.total_fallos > i.fallos.len() {
                t.push_str(&format!(
                    "      ... y {} fallos mas\n",
                    i.total_fallos - i.fallos.len()
                ));
            }
        }
        let fallos = self.total_fallos();
        t.push_str(&format!(
            "{} instrucciones, {} comprobaciones, {} fallos\n",
            self.instrucciones.len(),
            self.total_casos(),
            fallos
        ));
        t
    }
}

/// Patron de relleno que el emulador y la referencia ven en la memoria que nadie escribio.
fn relleno(direccion: u64, mascara: u64) -> u64 {
    (direccion.wrapping_mul(131).wrapping_add(17) ^ 0xA5) & mascara
}

fn mascara(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

enum Salida {
    Hecho(Vec<(Verificacion, Vec<String>)>),
    Omitido,
}

struct Contexto<'m> {
    maquina: &'m Maquina,
    referencia: Referencia<'m>,
    emulador: Emulador,
    inicial: lorenz_emu::Instantanea,
    direccion: u64,
}

impl<'m> Contexto<'m> {
    fn nuevo(maquina: &'m Maquina) -> Result<Self, String> {
        let region = region_de_pruebas(maquina).ok_or_else(|| {
            "la maquina no tiene una region de RAM de al menos 64 palabras".to_string()
        })?;
        let emulador = Emulador::nuevo(maquina.clone()).map_err(|e| e.to_string())?;
        let inicial = emulador.instantanea();
        Ok(Contexto {
            maquina,
            referencia: Referencia::nueva(maquina),
            emulador,
            inicial,
            direccion: region.desde + 8,
        })
    }

    fn ejecutar_semantica(&mut self, caso: &Caso) -> Result<Salida, String> {
        let m = self.maquina;
        let ins = &m.instrucciones[caso.indice];
        let valores = valores_de_codificacion(ins, &caso.operandos, self.direccion);
        let Ok(palabras) = codificar(m, ins, &valores, self.direccion) else {
            return Ok(Salida::Omitido);
        };
        let longitud = palabras.len() as u64;
        let m_dir = mascara(m.bits_direccion);
        let siguiente = self.direccion.wrapping_add(longitud) & m_dir;
        let m_pal = mascara(m.bits_palabra);
        let direccion = self.direccion;
        let en_instruccion = |a: u64| a >= direccion && a < direccion + longitud;

        // Estado de la referencia.
        let mut estado: EstadoRef = self.referencia.estado_inicial();
        for (n, v) in &caso.patron.registros {
            estado.registros.insert(n.clone(), *v);
        }
        for (n, v) in &caso.patron.banderas {
            estado.banderas.insert(n.clone(), *v);
        }
        estado.ie = caso.patron.ie && m.interrupciones.is_some();
        let operandos: Vec<ValorDeOperando> = caso
            .operandos
            .iter()
            .map(|(n, v)| ValorDeOperando {
                nombre: n.clone(),
                valor: *v,
            })
            .collect();
        let memoria = |a: u64| {
            if en_instruccion(a) {
                palabras[(a - direccion) as usize]
            } else {
                relleno(a, m_pal)
            }
        };
        let previo = estado.clone();
        let referencia = self
            .referencia
            .ejecutar(ins, &operandos, estado, siguiente, &memoria);
        let referencia = match referencia {
            Err(ErrorRef::UsaPuertos) => return Ok(Salida::Omitido),
            Ok(r) => {
                if r.estado.accedidas.iter().any(|a| !es_ram(m, *a)) {
                    return Ok(Salida::Omitido);
                }
                Some(r)
            }
            Err(_) => None,
        };

        // Estado del emulador.
        self.emulador
            .restaurar(&self.inicial)
            .map_err(|e| e.to_string())?;
        for (i, p) in palabras.iter().enumerate() {
            self.emulador
                .memoria_mut()
                .escribir_privilegiado(direccion + i as u64, *p)
                .map_err(|e| e.to_string())?;
        }
        if let Some(r) = &referencia {
            for a in &r.estado.lecturas_iniciales {
                if !en_instruccion(*a) {
                    self.emulador
                        .memoria_mut()
                        .escribir_privilegiado(*a, relleno(*a, m_pal))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        {
            let e = self.emulador.estado_mut();
            for (n, v) in &caso.patron.registros {
                e.fijar_registro(n, *v).map_err(|x| x.to_string())?;
            }
            for (n, v) in &caso.patron.banderas {
                e.fijar_bandera(n, *v).map_err(|x| x.to_string())?;
            }
            if m.interrupciones.is_some() {
                e.habilitar_interrupciones(caso.patron.ie);
            }
            e.fijar_pc(direccion);
        }
        self.emulador.memoria_mut().registrar_escrituras(true);
        let paso = self.emulador.paso();
        let escrituras = self.emulador.memoria_mut().tomar_escrituras();
        self.emulador.memoria_mut().registrar_escrituras(false);

        let mut hallazgos: Vec<(Verificacion, Vec<String>)> = Vec::new();
        let mut diferencias: Vec<String> = Vec::new();
        match (&referencia, &paso) {
            (None, Err(_)) => {}
            (None, Ok(_)) => {
                diferencias.push("la referencia rechaza la instruccion y el emulador no".into())
            }
            (Some(_), Err(e)) => diferencias.push(format!("el emulador falla: {e}")),
            (Some(r), Ok(p)) => {
                let Some(p) = p else {
                    return Err("el emulador estaba detenido al empezar".into());
                };
                self.comparar(r, p, &escrituras, &mut diferencias);
            }
        }
        if !diferencias.is_empty() {
            hallazgos.push((Verificacion::Semantica, diferencias));
        }

        // Banderas que cambiaron sin que la instruccion las declare.
        if paso.is_ok() {
            let asignadas = banderas_asignadas(ins, m);
            let mut ajenas = Vec::new();
            for b in &m.banderas {
                let antes = previo.banderas.get(&b.nombre).copied().unwrap_or(false);
                let despues = self
                    .emulador
                    .estado()
                    .bandera(&b.nombre)
                    .map_err(|e| e.to_string())?;
                if antes != despues
                    && !ins.afecta.contains(&b.nombre)
                    && !asignadas.contains(&b.nombre)
                {
                    ajenas.push(format!(
                        "la bandera {} paso de {} a {} sin estar en `afecta`",
                        b.nombre, antes as u8, despues as u8
                    ));
                }
            }
            if !ajenas.is_empty() {
                hallazgos.push((Verificacion::BanderasAjenas, ajenas));
            }
        }
        Ok(Salida::Hecho(hallazgos))
    }

    fn comparar(
        &self,
        r: &crate::referencia::ResultadoRef,
        p: &lorenz_emu::Paso,
        escrituras: &[lorenz_emu::EscrituraRegistrada],
        d: &mut Vec<String>,
    ) {
        let e = self.emulador.estado();
        for (n, esperado) in &r.estado.registros {
            match e.registro(n) {
                Ok(v) if v == *esperado => {}
                Ok(v) => d.push(format!(
                    "registro {n}: esperado {esperado:#x}, obtenido {v:#x}"
                )),
                Err(x) => d.push(format!("registro {n}: {x}")),
            }
        }
        if let Some(f) = self.maquina.registro_con_rol(RolRegistro::Banderas) {
            let esperado = self.referencia.banderas_empaquetadas(&r.estado);
            match e.registro(&f.nombre) {
                Ok(v) if v == esperado => {}
                Ok(v) => d.push(format!(
                    "registro {}: esperado {esperado:#x}, obtenido {v:#x}",
                    f.nombre
                )),
                Err(x) => d.push(format!("registro {}: {x}", f.nombre)),
            }
        }
        for (n, esperado) in &r.estado.banderas {
            match e.bandera(n) {
                Ok(v) if v == *esperado => {}
                Ok(v) => d.push(format!(
                    "bandera {n}: esperado {}, obtenido {}",
                    *esperado as u8, v as u8
                )),
                Err(x) => d.push(format!("bandera {n}: {x}")),
            }
        }
        if self.maquina.interrupciones.is_some() && e.interrupciones_habilitadas() != r.estado.ie {
            d.push(format!(
                "ie: esperado {}, obtenido {}",
                r.estado.ie as u8,
                e.interrupciones_habilitadas() as u8
            ));
        }
        if p.detenida != r.estado.detenida {
            d.push(format!(
                "detenida: esperado {}, obtenida {}",
                r.estado.detenida, p.detenida
            ));
        }
        if p.salto != r.estado.salto {
            d.push(format!(
                "salto: esperado {}, obtenido {}",
                r.estado.salto, p.salto
            ));
        }
        if p.ciclos != r.ciclos {
            d.push(format!(
                "ciclos: esperados {}, obtenidos {}",
                r.ciclos, p.ciclos
            ));
        }
        for (a, v) in &r.estado.escrituras {
            match self.emulador.memoria().leer_privilegiado(*a) {
                Ok(x) if x == *v => {}
                Ok(x) => d.push(format!("mem[{a:#x}]: esperado {v:#x}, obtenido {x:#x}")),
                Err(x) => d.push(format!("mem[{a:#x}]: {x}")),
            }
        }
        for w in escrituras {
            if !r.estado.escrituras.contains_key(&w.direccion) {
                d.push(format!(
                    "mem[{:#x}]: el emulador escribio {:#x} y la referencia no",
                    w.direccion, w.despues
                ));
            }
        }
    }
}

/// Texto de la direccion para el ensamblador: siempre como numero.
fn fuente_de(direccion: u64, texto: &str) -> Fuente {
    Fuente::nueva("conformidad.s", format!(".org {direccion:#x}\n{texto}\n"))
}

/// Ensambla, desensambla y decodifica las palabras de un caso y dice que no coincide.
fn ida_y_vuelta(
    m: &Maquina,
    desens: &Desensamblador<'_>,
    ens: &Ensamblador<'_>,
    dec: &Decodificador,
    caso: &Caso,
    direccion: u64,
) -> Vec<String> {
    let ins = &m.instrucciones[caso.indice];
    let valores = valores_de_codificacion(ins, &caso.operandos, direccion);
    let Ok(palabras) = codificar(m, ins, &valores, direccion) else {
        return Vec::new();
    };
    let mut d = Vec::new();
    // Decodificar debe recuperar la instruccion y sus operandos.
    match dec.decodificar_palabras(direccion, &palabras) {
        Ok(x) => {
            if x.indice != caso.indice {
                d.push(format!(
                    "decodifica como `{}` en vez de `{}`",
                    m.instrucciones[x.indice].mnemonico, ins.mnemonico
                ));
            } else {
                for (n, v) in &caso.operandos {
                    match x.operandos.iter().find(|o| &o.nombre == n) {
                        Some(o) if o.valor == *v => {}
                        Some(o) => d.push(format!(
                            "operando {n}: puesto {v}, decodificado {}",
                            o.valor
                        )),
                        None => d.push(format!("el operando {n} no se decodifica")),
                    }
                }
            }
        }
        Err(e) => d.push(format!("no se decodifica: {e}")),
    }
    // Desensamblar y volver a ensamblar debe dar las mismas palabras.
    let lineas = desens.desensamblar(direccion, &palabras, None);
    let texto = match lineas.first() {
        Some(Linea::Instruccion {
            texto,
            palabras: p,
            indice,
            ..
        }) if p == &palabras && *indice == caso.indice => texto.clone(),
        Some(otra) => {
            d.push(format!("el desensamblador devuelve otra cosa: {otra:?}"));
            return d;
        }
        None => {
            d.push("el desensamblador no devuelve nada".into());
            return d;
        }
    };
    match ens.ensamblar(&fuente_de(direccion, &texto)) {
        Ok(o) => {
            let nuevas: Vec<u64> = o.palabras().into_iter().map(|(_, w)| w).collect();
            if nuevas != palabras {
                d.push(format!(
                    "`{texto}` se reensambla como {nuevas:x?} y no como {palabras:x?}"
                ));
            }
        }
        Err(es) => d.push(format!(
            "`{texto}` no se reensambla: {}",
            es.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )),
    }
    d
}

/// Los valores fuera de rango deben rechazarse al codificar.
fn limites(m: &Maquina, indice: usize, direccion: u64, base: &[(String, i128)]) -> Vec<String> {
    let ins = &m.instrucciones[indice];
    let mut d = Vec::new();
    for o in &ins.operandos {
        for malo in fuera_de_rango(m, ins, o) {
            let mut operandos: Vec<(String, i128)> = base.to_vec();
            for (n, v) in operandos.iter_mut() {
                if n == &o.nombre {
                    *v = malo;
                }
            }
            let valores = valores_de_codificacion(ins, &operandos, direccion);
            if codificar(m, ins, &valores, direccion).is_ok() {
                d.push(format!(
                    "el operando {} acepta {malo}, que esta fuera de su rango",
                    o.nombre
                ));
            }
        }
    }
    d
}

/// Ejecuta la suite de conformidad completa sobre una maquina.
pub fn ejecutar_conformidad(
    maquina: &Maquina,
    opciones: &Opciones,
) -> Result<InformeConformidad, String> {
    let mut ctx = Contexto::nuevo(maquina)?;
    let desens = Desensamblador::nuevo(maquina)?;
    let ens = Ensamblador::nuevo(maquina)?;
    let dec = Decodificador::nuevo(maquina);
    let pats = patrones(maquina, opciones.patrones);
    let mut resultados = Vec::new();
    for (indice, ins) in maquina.instrucciones.iter().enumerate() {
        if let Some(m) = &opciones.mnemonico {
            if !ins.mnemonico.eq_ignore_ascii_case(m) {
                continue;
            }
        }
        let mut r = ResultadoInstruccion {
            mnemonico: ins.mnemonico.clone(),
            casos: 0,
            omitidos: 0,
            ida_y_vuelta: 0,
            limites: 0,
            total_fallos: 0,
            fallos: Vec::new(),
            nota: None,
        };
        let anotar =
            |r: &mut ResultadoInstruccion, v: Verificacion, caso: String, det: Vec<String>| {
                r.total_fallos += 1;
                if r.fallos.len() < MAX_FALLOS_DETALLADOS {
                    r.fallos.push(FalloConformidad {
                        verificacion: v,
                        caso,
                        detalles: det,
                    });
                }
            };
        if usa_puertos(ins) {
            r.nota = Some("la semantica usa puertos y no se compara".into());
        }
        let repeticiones = opciones.repeticiones.max(1);
        let casos = casos_de(
            maquina,
            indice,
            opciones.max_combinaciones,
            repeticiones,
            &pats,
        );
        for (n, caso) in casos.iter().enumerate() {
            if r.nota.is_none() {
                match ctx.ejecutar_semantica(caso)? {
                    Salida::Omitido => r.omitidos += 1,
                    Salida::Hecho(h) => {
                        r.casos += 1;
                        for (v, det) in h {
                            anotar(&mut r, v, caso.describir(maquina), det);
                        }
                    }
                }
            }
            if n % repeticiones == 0 {
                let d = ida_y_vuelta(maquina, &desens, &ens, &dec, caso, ctx.direccion);
                r.ida_y_vuelta += 1;
                if !d.is_empty() {
                    anotar(&mut r, Verificacion::IdaYVuelta, caso.describir(maquina), d);
                }
            }
        }
        if let Some(primero) = casos.first() {
            let d = limites(maquina, indice, ctx.direccion, &primero.operandos);
            r.limites += 1;
            if !d.is_empty() {
                anotar(&mut r, Verificacion::Limites, primero.describir(maquina), d);
            }
        }
        resultados.push(r);
    }
    if resultados.is_empty() {
        return Err(match &opciones.mnemonico {
            Some(m) => format!("la maquina no tiene la instruccion `{m}`"),
            None => "la maquina no tiene instrucciones".into(),
        });
    }
    Ok(InformeConformidad {
        formato: FORMATO_INFORME.into(),
        maquina: maquina.nombre.clone(),
        instrucciones: resultados,
    })
}
