//! Genericos sencillos sobre tipos enteros (RF-SGT-021).
//!
//! Una funcion puede declarar parametros de tipo, `funcion maximo<T>(a: T, b: T) -> T`, y se llama
//! diciendo con que tipos: `maximo<u8>(1, 2)`. Se resuelven por *monomorfizacion*, antes del
//! analisis de tipos: por cada combinacion de tipos con que se llama se genera una copia de la
//! funcion con los parametros de tipo sustituidos, de nombre `maximo__u8`, y la llamada pasa a ser
//! una llamada corriente a esa copia. El resto del compilador no sabe que existen los genericos.
//!
//! Los tipos con que se instancia deben ser enteros (`u8`, `i32`, `usize`, ...): lo que se
//! comprueba aqui es la forma del nombre, y el analisis comprueba despues que el ancho existe en
//! la maquina. No se infieren los tipos de los argumentos: hay que escribirlos. Una funcion
//! generica solo se puede llamar desde el archivo donde esta definida.

use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostico::Diagnostico;

/// Las instancias que se pueden crear como mucho: corta una cadena que no termina nunca.
pub const MAX_INSTANCIAS: usize = 1024;

/// Expande los genericos del programa principal y de cada modulo, por separado.
pub fn expandir_programa(archivo: &str, p: &Programa) -> Result<Programa, Vec<Diagnostico>> {
    let mut salida = p.clone();
    let mut errores = Vec::new();
    match expandir(archivo, std::mem::take(&mut salida.elementos)) {
        Ok(e) => salida.elementos = e,
        Err(mut d) => errores.append(&mut d),
    }
    for m in &mut salida.modulos {
        match expandir(&m.archivo, std::mem::take(&mut m.elementos)) {
            Ok(e) => m.elementos = e,
            Err(mut d) => errores.append(&mut d),
        }
    }
    if errores.is_empty() {
        Ok(salida)
    } else {
        Err(errores)
    }
}

/// Si el nombre tiene forma de tipo entero: `u8`, `i32`, `usize`.
fn es_nombre_de_entero(n: &str) -> bool {
    if n == "usize" {
        return true;
    }
    let Some(cifras) = n.strip_prefix('u').or_else(|| n.strip_prefix('i')) else {
        return false;
    };
    !cifras.is_empty() && cifras.chars().all(|c| c.is_ascii_digit())
}

/// Sustituye los parametros de tipo por los tipos dados, dentro de un tipo.
fn sustituir_tipo(t: &mut TipoAst, m: &HashMap<String, TipoAst>) {
    match &mut t.clase {
        ClaseTipo::Nombre(n) => {
            if let Some(r) = m.get(n.as_str()) {
                *t = r.clone();
            }
        }
        ClaseTipo::Puntero(i) => sustituir_tipo(i, m),
        ClaseTipo::Arreglo(i, _) => sustituir_tipo(i, m),
    }
}

// --- recorrido ---

type FnExpr<'a> = &'a mut dyn FnMut(&mut Expr);
type FnTipo<'a> = &'a mut dyn FnMut(&mut TipoAst);

fn recorrer_expr(e: &mut Expr, fe: FnExpr<'_>, ft: FnTipo<'_>) {
    match &mut e.clase {
        ClaseExpr::Binario(_, a, b) | ClaseExpr::Indice(a, b) => {
            recorrer_expr(a, fe, ft);
            recorrer_expr(b, fe, ft);
        }
        ClaseExpr::Unario(_, a)
        | ClaseExpr::DireccionDe(a)
        | ClaseExpr::Deref(a)
        | ClaseExpr::Campo(a, _) => recorrer_expr(a, fe, ft),
        ClaseExpr::Llamada(_, args) => args.iter_mut().for_each(|a| recorrer_expr(a, fe, ft)),
        ClaseExpr::LlamadaGenerica(_, tipos, args) => {
            for t in tipos.iter_mut() {
                ft(t);
            }
            args.iter_mut().for_each(|a| recorrer_expr(a, fe, ft));
        }
        ClaseExpr::Cast(a, t) => {
            recorrer_expr(a, fe, ft);
            ft(t);
        }
        ClaseExpr::Entero(_) | ClaseExpr::Bool(_) | ClaseExpr::Cadena(_) | ClaseExpr::Nombre(_) => {
        }
    }
    fe(e);
}

fn recorrer_bloque(b: &mut Bloque, fe: FnExpr<'_>, ft: FnTipo<'_>) {
    for s in &mut b.sentencias {
        match &mut s.clase {
            ClaseSentencia::Var { tipo, inicial, .. } => {
                if let Some(t) = tipo {
                    ft(t);
                }
                if let Some(i) = inicial {
                    recorrer_expr(i, fe, ft);
                }
            }
            ClaseSentencia::Asignar { destino, valor, .. } => {
                recorrer_expr(destino, fe, ft);
                recorrer_expr(valor, fe, ft);
            }
            ClaseSentencia::Si {
                condicion,
                entonces,
                sino,
            } => {
                recorrer_expr(condicion, fe, ft);
                recorrer_bloque(entonces, fe, ft);
                if let Some(s) = sino {
                    recorrer_bloque(s, fe, ft);
                }
            }
            ClaseSentencia::Mientras { condicion, cuerpo } => {
                recorrer_expr(condicion, fe, ft);
                recorrer_bloque(cuerpo, fe, ft);
            }
            ClaseSentencia::Bucle(c) => recorrer_bloque(c, fe, ft),
            ClaseSentencia::Segun {
                valor,
                brazos,
                otro,
            } => {
                recorrer_expr(valor, fe, ft);
                for (v, c) in brazos {
                    recorrer_expr(v, fe, ft);
                    recorrer_bloque(c, fe, ft);
                }
                if let Some(o) = otro {
                    recorrer_bloque(o, fe, ft);
                }
            }
            ClaseSentencia::Devolver(e) => {
                if let Some(e) = e {
                    recorrer_expr(e, fe, ft);
                }
            }
            ClaseSentencia::Expr(e) => recorrer_expr(e, fe, ft),
            ClaseSentencia::Romper | ClaseSentencia::Continuar | ClaseSentencia::Asm(_) => {}
        }
    }
}

// --- expansion ---

struct Expansor<'a> {
    archivo: &'a str,
    genericas: HashMap<String, DefFuncion>,
    /// Nombres de las funciones que no son genericas, para detectar choques.
    corrientes: Vec<String>,
    /// `clave` (funcion y tipos) -> nombre de la instancia.
    instancias: HashMap<String, String>,
    pendientes: Vec<DefFuncion>,
    errores: Vec<Diagnostico>,
}

/// Expande los genericos de una lista de elementos: quita las definiciones genericas y agrega,
/// al final, una copia por cada instancia que se use.
pub fn expandir(
    archivo: &str,
    elementos: Vec<Elemento>,
) -> Result<Vec<Elemento>, Vec<Diagnostico>> {
    let mut genericas = HashMap::new();
    let mut resto = Vec::new();
    let mut errores = Vec::new();
    for e in elementos {
        match e {
            Elemento::Funcion(f) if !f.tipos.is_empty() => {
                if !f.atributos.is_empty() {
                    errores.push(
                        Diagnostico::nuevo(
                            archivo,
                            f.lugar,
                            format!(
                                "la funcion generica `{}` no puede llevar atributos",
                                f.nombre
                            ),
                        )
                        .con_sugerencia("quita el atributo o la parte generica".to_string()),
                    );
                }
                genericas.insert(f.nombre.clone(), f);
            }
            otro => resto.push(otro),
        }
    }
    let corrientes = resto
        .iter()
        .filter_map(|e| match e {
            Elemento::Funcion(f) => Some(f.nombre.clone()),
            _ => None,
        })
        .collect();
    let mut x = Expansor {
        archivo,
        genericas,
        corrientes,
        instancias: HashMap::new(),
        pendientes: Vec::new(),
        errores,
    };
    for e in &mut resto {
        match e {
            Elemento::Funcion(f) => x.recorrer_funcion(f),
            Elemento::Const(c) => x.recorrer_expr_suelta(&mut c.valor),
            Elemento::Global(g) => {
                if let Some(d) = &mut g.direccion {
                    x.recorrer_expr_suelta(d);
                }
                if let Some(i) = &mut g.inicial {
                    x.recorrer_expr_suelta(i);
                }
            }
            Elemento::Importar(_) | Elemento::Estructura(_) => {}
        }
    }
    // Las instancias pueden pedir otras: se procesan hasta que no quede ninguna.
    let mut hechas = Vec::new();
    while let Some(mut f) = x.pendientes.pop() {
        x.recorrer_funcion(&mut f);
        hechas.push(f);
    }
    if !x.errores.is_empty() {
        return Err(x.errores);
    }
    hechas.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    resto.extend(hechas.into_iter().map(Elemento::Funcion));
    Ok(resto)
}

impl Expansor<'_> {
    fn error(&mut self, lugar: crate::diagnostico::Lugar, mensaje: String, sugerencia: String) {
        self.errores
            .push(Diagnostico::nuevo(self.archivo, lugar, mensaje).con_sugerencia(sugerencia));
    }

    fn recorrer_funcion(&mut self, f: &mut DefFuncion) {
        let mut cuerpo = std::mem::take(&mut f.cuerpo);
        recorrer_bloque(&mut cuerpo, &mut |e| self.reescribir(e), &mut |_| {});
        f.cuerpo = cuerpo;
    }

    fn recorrer_expr_suelta(&mut self, e: &mut Expr) {
        recorrer_expr(e, &mut |e| self.reescribir(e), &mut |_| {});
    }

    /// Cambia una llamada generica por una llamada a su instancia, y senala las llamadas a una
    /// generica que no dicen sus tipos.
    fn reescribir(&mut self, e: &mut Expr) {
        match &mut e.clase {
            ClaseExpr::Llamada(n, _) if self.genericas.contains_key(n.as_str()) => {
                let (n, lugar) = (n.clone(), e.lugar);
                let ejemplo = self.genericas[&n]
                    .tipos
                    .iter()
                    .map(|_| "u8")
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error(
                    lugar,
                    format!(
                        "la funcion generica `{n}` necesita que se diga con que tipos se llama"
                    ),
                    format!("escribe `{n}<{ejemplo}>(...)`"),
                );
            }
            ClaseExpr::LlamadaGenerica(..) => {
                let ClaseExpr::LlamadaGenerica(n, tipos, args) =
                    std::mem::replace(&mut e.clase, ClaseExpr::Entero(0))
                else {
                    return;
                };
                match self.instanciar(e.lugar, &n, &tipos) {
                    Some(inst) => e.clase = ClaseExpr::Llamada(inst, args),
                    None => e.clase = ClaseExpr::Llamada(n, args),
                }
            }
            _ => {}
        }
    }

    /// El nombre de la copia de `n` para esos tipos; la crea si no existe.
    fn instanciar(
        &mut self,
        lugar: crate::diagnostico::Lugar,
        n: &str,
        tipos: &[TipoAst],
    ) -> Option<String> {
        let Some(def) = self.genericas.get(n) else {
            let m = if self.corrientes.iter().any(|c| c == n) {
                format!("`{n}` no es una funcion generica y no lleva tipos")
            } else {
                format!("no hay una funcion generica llamada `{n}`")
            };
            self.error(lugar, m, format!("declarala como `funcion {n}<T>(...)`"));
            return None;
        };
        if def.tipos.len() != tipos.len() {
            let m = format!(
                "`{n}` tiene {} parametro{} de tipo y se le dan {}",
                def.tipos.len(),
                if def.tipos.len() == 1 { "" } else { "s" },
                tipos.len()
            );
            self.error(lugar, m, format!("escribe `{n}<{}>`", def.tipos.join(", ")));
            return None;
        }
        let mut nombres = Vec::new();
        for t in tipos {
            match &t.clase {
                ClaseTipo::Nombre(x) if es_nombre_de_entero(x) => nombres.push(x.clone()),
                _ => {
                    self.error(
                        t.lugar,
                        "una funcion generica solo se instancia con tipos enteros".to_string(),
                        "usa un tipo como u8, u16, i32 o usize".to_string(),
                    );
                    return None;
                }
            }
        }
        let inst = format!("{n}__{}", nombres.join("_"));
        if self.instancias.contains_key(&inst) {
            return Some(inst);
        }
        if self.corrientes.contains(&inst) {
            self.error(
                lugar,
                format!("la copia `{inst}` de la funcion generica choca con otra funcion"),
                "cambia el nombre de una de las dos".to_string(),
            );
            return None;
        }
        if self.instancias.len() >= MAX_INSTANCIAS {
            self.error(
                lugar,
                format!("demasiadas copias de funciones genericas (mas de {MAX_INSTANCIAS})"),
                "revisa si una funcion generica se llama a si misma con tipos distintos"
                    .to_string(),
            );
            return None;
        }
        self.instancias.insert(inst.clone(), inst.clone());
        let mapa: HashMap<String, TipoAst> = def
            .tipos
            .iter()
            .cloned()
            .zip(tipos.iter().cloned())
            .collect();
        let mut copia = def.clone();
        copia.nombre = inst.clone();
        copia.tipos.clear();
        for p in &mut copia.parametros {
            sustituir_tipo(&mut p.tipo, &mapa);
        }
        if let Some(r) = &mut copia.retorno {
            sustituir_tipo(r, &mapa);
        }
        recorrer_bloque(
            &mut copia.cuerpo,
            &mut |e| match &mut e.clase {
                ClaseExpr::Cast(_, t) => sustituir_tipo(t, &mapa),
                ClaseExpr::LlamadaGenerica(_, ts, _) => {
                    ts.iter_mut().for_each(|t| sustituir_tipo(t, &mapa))
                }
                _ => {}
            },
            &mut |t| sustituir_tipo(t, &mapa),
        );
        self.pendientes.push(copia);
        Some(inst)
    }
}
