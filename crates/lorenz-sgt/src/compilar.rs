//! De codigo fuente SGT a programa para una maquina (RF-SGT-023).

use lorenz_arch::modelo::Maquina;
use lorenz_ens::ensamblador::{Ensamblador, Fuente, Objeto};

use crate::analisis::{analizar, Opciones};
use crate::costes::{comprobar_presupuestos, estimar, Coste};
use crate::destino::Destino;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::emisor::Linea;
use crate::generador::{generar, sustitucion_para};
use crate::hir::Modulo;
use crate::layout::Disposicion;
use crate::sintaxis::analizar_programa;

/// Nombre con el que se ensambla el texto generado.
pub const ARCHIVO_GENERADO: &str = "(generado).s";

#[derive(Debug, Clone)]
pub struct Compilado {
    pub modulo: Modulo,
    pub disposicion: Disposicion,
    pub lineas: Vec<Linea>,
    pub objeto: Objeto,
    /// Para cada tramo de codigo, la linea del fuente SGT que lo origino (RF-SGT-038).
    pub lineas_fuente: Vec<(u64, u32)>,
    /// Coste en ciclos de cada funcion (RF-SGT-030).
    pub costes: Vec<Coste>,
}

impl Compilado {
    /// El ensamblador generado.
    pub fn ensamblador(&self) -> String {
        self.lineas
            .iter()
            .map(|l| format!("{}\n", l.texto))
            .collect()
    }

    /// El ensamblador con, antes de cada grupo de instrucciones, la linea de SGT que lo origino
    /// (RF-SGT-032).
    pub fn ensamblador_comentado(&self, fuente: &str) -> String {
        let textos: Vec<&str> = fuente.lines().collect();
        let mut s = String::new();
        let mut ultima = None;
        for l in &self.lineas {
            if l.fuente != ultima {
                ultima = l.fuente;
                if let Some(n) = l.fuente {
                    let t = textos.get(n as usize - 1).map(|t| t.trim()).unwrap_or("");
                    s.push_str(&format!("; {n}: {t}\n"));
                }
            }
            s.push_str(&l.texto);
            s.push('\n');
        }
        s
    }

    /// Palabras del programa como `(direccion, palabra)`.
    pub fn palabras(&self) -> Vec<(u64, u64)> {
        self.objeto.palabras()
    }
}

pub fn compilar(
    archivo: &str,
    texto: &str,
    maquina: &Maquina,
    opciones: Opciones,
) -> Result<Compilado, Vec<Diagnostico>> {
    let programa = analizar_programa(archivo, texto).map_err(|d| vec![d])?;
    let destino = Destino::de(maquina);
    reservados(archivo, &programa)?;
    // La biblioteca de apoyo va primero; lo que no se use se poda despues.
    let mut completo = crate::biblioteca::programa(maquina.bits_palabra).map_err(|d| vec![d])?;
    completo
        .elementos
        .extend(programa.elementos.iter().cloned());
    let nivel = opciones.nivel.min(crate::optimizar::NIVEL_MAXIMO);
    let mut modulo = analizar(archivo, &completo, &destino, opciones)?;
    crate::optimizar::optimizar(&mut modulo, &destino, nivel);
    let modulo = crate::podar::podar(modulo, &sustitucion_para(maquina));
    // Primero con saltos cortos; si algun salto no alcanza, todo con saltos largos.
    let mut ultimo = None;
    for largos in [false, true] {
        let g = generar(&modulo, &destino, maquina, largos)?;
        let fuente = Fuente::nueva(
            ARCHIVO_GENERADO,
            g.lineas
                .iter()
                .map(|l| format!("{}\n", l.texto))
                .collect::<String>(),
        );
        let ens = Ensamblador::nuevo(maquina).map_err(|e| vec![interno(archivo, &e)])?;
        match ens.ensamblar(&fuente) {
            Ok(objeto) => {
                let mut lineas_fuente = Vec::new();
                for e in &objeto.mapa {
                    if let Some(l) = g.lineas.get(e.linea.saturating_sub(1)) {
                        if let Some(n) = l.fuente {
                            lineas_fuente.push((e.direccion, n));
                        }
                    }
                }
                let costes = estimar(&modulo, maquina, &g.lineas, &sustitucion_para(maquina));
                let mut excedidos = comprobar_presupuestos(&modulo, &costes);
                excedidos.dedup();
                if !excedidos.is_empty() {
                    return Err(excedidos);
                }
                return Ok(Compilado {
                    costes,
                    modulo,
                    disposicion: g.disposicion,
                    lineas: g.lineas,
                    objeto,
                    lineas_fuente,
                });
            }
            Err(errores) => {
                ultimo = Some((
                    errores
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; "),
                    g.lineas,
                ));
            }
        }
    }
    let (msg, lineas) = ultimo.unwrap();
    let texto: String = lineas.iter().map(|l| format!("{}\n", l.texto)).collect();
    Err(vec![interno(
        archivo,
        &format!("el ensamblador rechazo el codigo generado: {msg}\n--- ensamblador generado ---\n{texto}"),
    )])
}

fn interno(archivo: &str, msg: &str) -> Diagnostico {
    Diagnostico::nuevo(
        archivo,
        Lugar::nuevo(1, 1),
        format!("error interno del compilador: {msg}"),
    )
}

/// Los nombres que empiezan por `__` son del compilador.
fn reservados(archivo: &str, p: &crate::ast::Programa) -> Result<(), Vec<Diagnostico>> {
    use crate::ast::Elemento;
    let mut errores = Vec::new();
    for e in &p.elementos {
        let (nombre, lugar) = match e {
            Elemento::Funcion(f) => (&f.nombre, f.lugar),
            Elemento::Global(g) => (&g.nombre, g.lugar),
            Elemento::Const(c) => (&c.nombre, c.lugar),
            Elemento::Estructura(s) => (&s.nombre, s.lugar),
            Elemento::Importar(_) => continue,
        };
        if crate::biblioteca::es_reservado(nombre) {
            errores.push(
                Diagnostico::nuevo(archivo, lugar, format!("el nombre `{nombre}` esta reservado: los nombres que empiezan por `__` son del compilador"))
                    .con_sugerencia("elige otro nombre"),
            );
        }
    }
    if errores.is_empty() {
        Ok(())
    } else {
        Err(errores)
    }
}
