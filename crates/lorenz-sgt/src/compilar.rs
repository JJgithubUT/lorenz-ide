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

/// Busca el texto de un modulo por su nombre: devuelve el archivo de donde sale y su contenido.
pub type Cargador<'a> = &'a dyn Fn(&str) -> Option<(String, String)>;

/// Un cargador que busca cada modulo como `nombre.sgt` en la carpeta del archivo que se compila.
pub fn en_carpeta_de(archivo: &str) -> impl Fn(&str) -> Option<(String, String)> {
    let carpeta = std::path::Path::new(archivo)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    move |modulo: &str| {
        let ruta = carpeta.join(format!("{modulo}.sgt"));
        let texto = std::fs::read_to_string(&ruta).ok()?;
        Some((ruta.to_string_lossy().into_owned(), texto))
    }
}

/// Compila un solo archivo: `importar` no encuentra ningun modulo.
pub fn compilar(
    archivo: &str,
    texto: &str,
    maquina: &Maquina,
    opciones: Opciones,
) -> Result<Compilado, Vec<Diagnostico>> {
    compilar_con(archivo, texto, maquina, opciones, &|_| None)
}

/// Compila un programa que puede importar modulos (RF-SGT-018); `cargador` los busca por nombre.
pub fn compilar_con(
    archivo: &str,
    texto: &str,
    maquina: &Maquina,
    opciones: Opciones,
    cargador: Cargador<'_>,
) -> Result<Compilado, Vec<Diagnostico>> {
    let programa = analizar_programa(archivo, texto).map_err(|d| vec![d])?;
    let destino = Destino::de(maquina);
    reservados(archivo, &programa)?;
    let modulos = cargar_modulos(&programa, cargador)?;
    // La biblioteca de apoyo va primero; lo que no se use se poda despues.
    let mut completo = crate::biblioteca::programa(maquina.bits_palabra).map_err(|d| vec![d])?;
    completo
        .elementos
        .extend(programa.elementos.iter().cloned());
    completo.modulos = modulos;
    let nivel = opciones.nivel.min(crate::optimizar::NIVEL_MAXIMO);
    let mut modulo = analizar(archivo, &completo, &destino, opciones)?;
    crate::multipalabra::bajar(&mut modulo, &destino)?;
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

/// Carga, de forma transitiva, los modulos que importa el programa. Cada uno queda despues de los
/// que importa. Los que el cargador no encuentra los reporta el analizador.
fn cargar_modulos(
    programa: &crate::ast::Programa,
    cargador: Cargador<'_>,
) -> Result<Vec<crate::ast::ModuloFuente>, Vec<Diagnostico>> {
    let mut hechos: Vec<crate::ast::ModuloFuente> = Vec::new();
    let mut en_curso: Vec<String> = Vec::new();
    cargar_de(
        &programa.elementos,
        "",
        cargador,
        &mut hechos,
        &mut en_curso,
    )?;
    Ok(hechos)
}

fn cargar_de(
    elementos: &[crate::ast::Elemento],
    archivo: &str,
    cargador: Cargador<'_>,
    hechos: &mut Vec<crate::ast::ModuloFuente>,
    en_curso: &mut Vec<String>,
) -> Result<(), Vec<Diagnostico>> {
    use crate::ast::Elemento;
    for e in elementos {
        let Elemento::Importar(i) = e else { continue };
        if hechos.iter().any(|m| m.nombre == i.modulo) {
            continue;
        }
        if en_curso.contains(&i.modulo) {
            let mut ciclo = en_curso.clone();
            ciclo.push(i.modulo.clone());
            return Err(vec![Diagnostico::nuevo(
                archivo,
                i.lugar,
                format!("los modulos se importan en ciclo: {}", ciclo.join(" -> ")),
            )
            .con_sugerencia(
                "saca lo que comparten a un tercer modulo".to_string(),
            )]);
        }
        let Some((ruta, texto)) = cargador(&i.modulo) else {
            continue;
        };
        let modulo = analizar_programa(&ruta, &texto).map_err(|d| vec![d])?;
        reservados(&ruta, &modulo)?;
        en_curso.push(i.modulo.clone());
        cargar_de(&modulo.elementos, &ruta, cargador, hechos, en_curso)?;
        en_curso.pop();
        hechos.push(crate::ast::ModuloFuente {
            nombre: i.modulo.clone(),
            archivo: ruta,
            elementos: modulo.elementos,
        });
    }
    Ok(())
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
                Diagnostico::nuevo(archivo, lugar, if nombre.starts_with("__") {
                    format!("el nombre `{nombre}` esta reservado: los nombres que empiezan por `__` son del compilador")
                } else {
                    format!("el nombre `{nombre}` es de la biblioteca de SGT y no se puede redefinir")
                })
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
