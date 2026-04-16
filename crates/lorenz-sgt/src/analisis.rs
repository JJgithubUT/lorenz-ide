//! Analisis semantico: nombres, tipos, constantes y comprobaciones contra la maquina destino.
//!
//! Convierte el arbol sintactico en la representacion tipada de [`crate::hir`]. Sigue despues de un
//! error para dar todos los que pueda, pero un tipo que ya fallo se marca como `Error` y no vuelve a
//! protestar. Los mensajes hablan de la maquina destino: anchos de palabra, regiones, puertos
//! (RF-SGT-035).

use std::collections::{BTreeSet, HashMap};

use crate::ast::*;
use crate::destino::Destino;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::hir::{self, ClaseExpr as CE, ClaseSent, ClaseSitio, Ref, Sitio};
use crate::tipos::{envolver, rango, Tipo};
use crate::valores;

/// Ajustes del analisis.
#[derive(Debug, Clone, Copy)]
pub struct Opciones {
    /// Si falta `principal` es un error; las bibliotecas lo desactivan.
    pub exige_principal: bool,
    /// Nivel de optimizacion, de 0 a 3 (RF-SGT-033).
    pub nivel: u8,
}

impl Default for Opciones {
    fn default() -> Self {
        Self {
            exige_principal: true,
            nivel: 1,
        }
    }
}

/// Nombre de la funcion donde empieza el programa.
pub const PRINCIPAL: &str = "principal";

pub fn analizar(
    archivo: &str,
    programa: &Programa,
    destino: &Destino,
    opciones: Opciones,
) -> Result<hir::Modulo, Vec<Diagnostico>> {
    let programa = &crate::genericos::expandir_programa(archivo, programa)?;
    let mut a = Analizador::nuevo(archivo, destino);
    a.programa(programa, opciones);
    if a.diag.is_empty() {
        Ok(a.modulo)
    } else {
        a.diag.sort_by_key(|d| d.lugar);
        Err(a.diag)
    }
}

#[derive(Clone)]
struct Firma {
    parametros: Vec<Tipo>,
    retorno: Tipo,
    manejador: bool,
}

struct Analizador<'a> {
    archivo: String,
    destino: &'a Destino,
    /// Modulo que se analiza (vacio para el programa) y los simbolos importados por cada uno.
    mod_actual: String,
    alias: HashMap<(String, String), String>,
    diag: Vec<Diagnostico>,
    modulo: hir::Modulo,
    estructuras: HashMap<String, usize>,
    consts: HashMap<String, (Tipo, i128)>,
    globales: HashMap<String, hir::IdGlobal>,
    funciones: HashMap<String, hir::IdFuncion>,
    firmas: Vec<Firma>,
    // Estado de la funcion que se analiza.
    locales: Vec<hir::Variable>,
    ambitos: Vec<HashMap<String, hir::IdLocal>>,
    retorno: Tipo,
    bucles: u32,
    llamadas: Vec<(hir::IdFuncion, Lugar)>,
}

/// La conversion que arregla un error de tipos, si existe y es segura de proponer.
fn conversion_sugerida(origen: &Tipo, destino: &Tipo) -> Option<String> {
    match (origen, destino) {
        (Tipo::Entero { .. }, Tipo::Entero { .. })
        | (Tipo::Bool, Tipo::Entero { .. })
        | (Tipo::Puntero(_), Tipo::Puntero(_)) => {
            Some(format!("convierte el valor con `como {destino}`"))
        }
        (Tipo::Entero { .. }, Tipo::Bool) => {
            Some("compara con `!= 0` para obtener un booleano".to_string())
        }
        _ => None,
    }
}

fn distancia(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut fila: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut anterior = fila[0];
        fila[0] = i;
        for j in 1..=b.len() {
            let tmp = fila[j];
            fila[j] = (fila[j] + 1)
                .min(fila[j - 1] + 1)
                .min(anterior + usize::from(a[i - 1] != b[j - 1]));
            anterior = tmp;
        }
    }
    fila[b.len()]
}

impl<'a> Analizador<'a> {
    fn nuevo(archivo: &'a str, destino: &'a Destino) -> Self {
        Self {
            archivo: archivo.to_string(),
            destino,
            mod_actual: String::new(),
            alias: HashMap::new(),
            diag: Vec::new(),
            modulo: hir::Modulo {
                archivo: archivo.to_string(),
                ..Default::default()
            },
            estructuras: HashMap::new(),
            consts: HashMap::new(),
            globales: HashMap::new(),
            funciones: HashMap::new(),
            firmas: Vec::new(),
            locales: Vec::new(),
            ambitos: Vec::new(),
            retorno: Tipo::Vacio,
            bucles: 0,
            llamadas: Vec::new(),
        }
    }

    fn error(&mut self, lugar: Lugar, mensaje: impl Into<String>) {
        self.diag
            .push(Diagnostico::nuevo(&self.archivo, lugar, mensaje));
    }

    /// Nombre unico de un simbolo de un modulo en el programa completo.
    fn nombre_en(modulo: &str, n: &str) -> String {
        if modulo.is_empty() {
            n.to_string()
        } else {
            format!("{modulo}__{n}")
        }
    }

    fn existe_simbolo(&self, k: &str) -> bool {
        self.estructuras.contains_key(k)
            || self.consts.contains_key(k)
            || self.globales.contains_key(k)
            || self.funciones.contains_key(k)
    }

    /// El nombre unico al que se refiere `n` desde el modulo que se analiza: un simbolo propio,
    /// uno importado o, desde un modulo, uno de la biblioteca de SGT.
    fn clave(&self, n: &str) -> String {
        let propio = Self::nombre_en(&self.mod_actual, n);
        if self.existe_simbolo(&propio) {
            return propio;
        }
        if let Some(a) = self.alias.get(&(self.mod_actual.clone(), n.to_string())) {
            return a.clone();
        }
        if !self.mod_actual.is_empty()
            && crate::biblioteca::es_reservado(n)
            && self.existe_simbolo(n)
        {
            return n.to_string();
        }
        propio
    }

    fn error_con(&mut self, lugar: Lugar, mensaje: impl Into<String>, sugerencia: String) {
        self.diag
            .push(Diagnostico::nuevo(&self.archivo, lugar, mensaje).con_sugerencia(sugerencia));
    }

    /// Un error de tipos: si hay una conversion que lo arregla, la sugiere (RF-SGT-037).
    fn error_tipos(&mut self, lugar: Lugar, mensaje: String, origen: &Tipo, destino: &Tipo) {
        match conversion_sugerida(origen, destino) {
            Some(s) => self.error_con(lugar, mensaje, s),
            None => self.error(lugar, mensaje),
        }
    }

    // ------------------------------------------------------------------
    // Tipos
    // ------------------------------------------------------------------

    fn palabras_de(&self, t: &Tipo) -> u64 {
        match t {
            Tipo::Entero { bits, .. } => self.destino.palabras_de(*bits),
            Tipo::Bool => 1,
            Tipo::Puntero(_) => self.destino.palabras_de(self.destino.direccion),
            Tipo::Arreglo(e, n) => self.palabras_de(e) * n,
            Tipo::Estructura(i) => self.modulo.estructuras[*i].tamano,
            Tipo::Vacio | Tipo::Error => 0,
        }
    }

    fn lista_de_enteros(&self) -> String {
        let mut v = Vec::new();
        for b in self.destino.anchos_de_entero() {
            v.push(format!("u{b}"));
            v.push(format!("i{b}"));
        }
        v.join(", ")
    }

    fn tipo_por_nombre(&mut self, nombre: &str, lugar: Lugar) -> Tipo {
        if nombre == "bool" {
            return Tipo::Bool;
        }
        if nombre == "usize" {
            return Tipo::u(self.destino.direccion);
        }
        if let Some(&i) = self.estructuras.get(&self.clave(nombre)) {
            return Tipo::Estructura(i);
        }
        let forma = nombre
            .strip_prefix('u')
            .map(|r| (r, false))
            .or_else(|| nombre.strip_prefix('i').map(|r| (r, true)));
        if let Some((cifras, signo)) = forma {
            if let Ok(bits) = cifras.parse::<u32>() {
                if self.destino.anchos_de_entero().contains(&bits) {
                    return Tipo::Entero { bits, signo };
                }
                let m = format!(
                    "el tipo `{nombre}` no existe en `{}`: sus palabras son de {} bits, asi que los enteros son {}",
                    self.destino.nombre,
                    self.destino.palabra,
                    self.lista_de_enteros()
                );
                let d = Diagnostico::nuevo(&self.archivo, lugar, m);
                // Una cifra casi igual a un ancho que existe (u61 por u16).
                let cercano = self
                    .destino
                    .anchos_de_entero()
                    .into_iter()
                    .map(|b| (b, format!("{}{b}", &nombre[..1])))
                    .filter(|(_, c)| distancia(c, nombre) <= 2)
                    .min_by_key(|(_, c)| {
                        // Las cifras cambiadas de orden (u61) van antes que las demas.
                        let (mut a, mut b): (Vec<char>, Vec<char>) =
                            (c.chars().collect(), nombre.chars().collect());
                        a.sort_unstable();
                        b.sort_unstable();
                        (a != b, distancia(c, nombre))
                    });
                self.diag.push(match cercano {
                    Some((_, c)) => d.con_sugerencia(format!("quiza querias decir `{c}`")),
                    None => d,
                });
                return Tipo::Error;
            }
        }
        let mut candidatos: Vec<String> = ["bool".to_string(), "usize".to_string()]
            .into_iter()
            .chain(self.estructuras.keys().cloned())
            .collect();
        for b in self.destino.anchos_de_entero() {
            candidatos.push(format!("u{b}"));
            candidatos.push(format!("i{b}"));
        }
        let cercano = candidatos
            .iter()
            .filter(|c| distancia(c, nombre) <= 2)
            .min_by_key(|c| distancia(c, nombre))
            .cloned();
        let d = Diagnostico::nuevo(
            &self.archivo,
            lugar,
            format!("no existe el tipo `{nombre}`"),
        );
        self.diag.push(match cercano {
            Some(c) => d.con_sugerencia(format!("quiza querias decir `{c}`")),
            None => d,
        });
        Tipo::Error
    }

    fn resolver_tipo(&mut self, t: &TipoAst) -> Tipo {
        match &t.clase {
            ClaseTipo::Nombre(n) => self.tipo_por_nombre(n, t.lugar),
            ClaseTipo::Puntero(i) => {
                let i = self.resolver_tipo(i);
                if i.is_error_or_vacio() {
                    Tipo::Error
                } else {
                    Tipo::Puntero(Box::new(i))
                }
            }
            ClaseTipo::Arreglo(e, n) => {
                let elemento = self.resolver_tipo(e);
                let largo = self.constante_entera(n, "el tamano de un arreglo");
                match (elemento, largo) {
                    (Tipo::Error, _) | (_, None) => Tipo::Error,
                    (_, Some(0)) => {
                        self.error(n.lugar, "un arreglo no puede tener tamano cero");
                        Tipo::Error
                    }
                    (e, Some(l)) => Tipo::Arreglo(Box::new(e), l as u64),
                }
            }
        }
    }

    /// Evalua una expresion que debe ser una constante entera no negativa.
    fn constante_entera(&mut self, e: &Expr, que: &str) -> Option<i128> {
        let h = self.expr(e, Some(&Tipo::u(64.min(self.destino.palabra * 4))));
        if h.tipo.es_error() {
            return None;
        }
        match h.clase {
            CE::Const(v) if v >= 0 => Some(v),
            CE::Const(_) => {
                self.error(e.lugar, format!("{que} no puede ser negativo"));
                None
            }
            _ => {
                self.error_con(
                    e.lugar,
                    format!("{que} debe ser una constante conocida al compilar"),
                    "usa un numero o una `const`".to_string(),
                );
                None
            }
        }
    }

    // ------------------------------------------------------------------
    // Programa
    // ------------------------------------------------------------------

    /// Anota que simbolos importa cada modulo y comprueba que existan (RF-SGT-018).
    fn registrar_importaciones(&mut self, unidades: &[(String, String, &[Elemento])]) {
        use std::collections::BTreeSet;
        let mut definidos: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (modulo, _, elementos) in unidades {
            let nombres = definidos.entry(modulo.clone()).or_default();
            for e in *elementos {
                match e {
                    Elemento::Const(c) => nombres.insert(c.nombre.clone()),
                    Elemento::Global(g) => nombres.insert(g.nombre.clone()),
                    Elemento::Estructura(d) => nombres.insert(d.nombre.clone()),
                    Elemento::Funcion(f) => nombres.insert(f.nombre.clone()),
                    Elemento::Importar(_) => false,
                };
            }
        }
        for (modulo, archivo, elementos) in unidades {
            self.archivo = archivo.clone();
            for e in *elementos {
                let Elemento::Importar(i) = e else { continue };
                let Some(propios) = definidos.get(&i.modulo) else {
                    self.error_con(
                        i.lugar,
                        format!("no se encontro el modulo `{}`", i.modulo),
                        format!("crea `{}.sgt` junto al archivo que lo importa", i.modulo),
                    );
                    continue;
                };
                if i.modulo.is_empty() || i.modulo == *modulo {
                    self.error(i.lugar, "un modulo no puede importarse a si mismo");
                    continue;
                }
                for s in &i.simbolos {
                    if !propios.contains(s) {
                        let tiene = if propios.is_empty() {
                            "no define nada".to_string()
                        } else {
                            format!(
                                "define: {}",
                                propios.iter().cloned().collect::<Vec<_>>().join(", ")
                            )
                        };
                        self.error(
                            i.lugar,
                            format!("el modulo `{}` no define `{s}` ({tiene})", i.modulo),
                        );
                        continue;
                    }
                    let clave = (modulo.clone(), s.clone());
                    if self.alias.contains_key(&clave) {
                        self.error(i.lugar, format!("`{s}` ya esta importado"));
                        continue;
                    }
                    self.alias.insert(clave, Self::nombre_en(&i.modulo, s));
                }
            }
        }
    }

    fn programa(&mut self, p: &Programa, opciones: Opciones) {
        // Los modulos importados van primero y el programa al final; los simbolos de cada modulo
        // llevan su nombre como prefijo (RF-SGT-018).
        let raiz = self.archivo.clone();
        let mut unidades: Vec<(String, String, &[Elemento])> = p
            .modulos
            .iter()
            .map(|m| (m.nombre.clone(), m.archivo.clone(), &m.elementos[..]))
            .collect();
        unidades.push((String::new(), raiz.clone(), &p.elementos[..]));
        self.registrar_importaciones(&unidades);
        // 1. Nombres de estructuras, para que se puedan citar entre ellas.
        let mut defs = Vec::new();
        for (modulo, archivo, elementos) in &unidades {
            self.mod_actual = modulo.clone();
            self.archivo = archivo.clone();
            for e in *elementos {
                if let Elemento::Estructura(d) = e {
                    let clave = Self::nombre_en(&self.mod_actual, &d.nombre);
                    if self.estructuras.contains_key(&clave)
                        || self
                            .alias
                            .contains_key(&(self.mod_actual.clone(), d.nombre.clone()))
                    {
                        self.error(
                            d.lugar,
                            format!("la estructura `{}` ya esta declarada", d.nombre),
                        );
                        continue;
                    }
                    let i = self.modulo.estructuras.len();
                    self.estructuras.insert(clave.clone(), i);
                    self.modulo.estructuras.push(hir::Estructura {
                        nombre: clave,
                        campos: Vec::new(),
                        tamano: 0,
                    });
                    defs.push((modulo.clone(), archivo.clone(), i, d));
                }
            }
        }
        // Las constantes se definen antes de disponer las estructuras: el desplazamiento de un
        // campo (`@ n`) y la longitud de un arreglo pueden nombrarlas (RF-SGT-007).
        for (modulo, archivo, elementos) in &unidades {
            self.mod_actual = modulo.clone();
            self.archivo = archivo.clone();
            for e in *elementos {
                if let Elemento::Const(c) = e {
                    self.def_const(c);
                }
            }
        }
        for (modulo, archivo, i, d) in defs {
            self.mod_actual = modulo;
            self.archivo = archivo;
            self.disponer_estructura(i, d);
        }
        // 2. Variables globales, en orden de aparicion.
        for (modulo, archivo, elementos) in &unidades {
            self.mod_actual = modulo.clone();
            self.archivo = archivo.clone();
            for e in *elementos {
                if let Elemento::Global(g) = e {
                    self.def_global(g);
                }
            }
        }
        // 3. Firmas de funciones.
        let mut cuerpos = Vec::new();
        for (modulo, archivo, elementos) in &unidades {
            self.mod_actual = modulo.clone();
            self.archivo = archivo.clone();
            for e in *elementos {
                if let Elemento::Funcion(f) = e {
                    if let Some(id) = self.firma(f) {
                        cuerpos.push((modulo.clone(), archivo.clone(), id, f));
                    }
                }
            }
        }
        // 4. Cuerpos.
        for (modulo, archivo, id, f) in cuerpos {
            self.mod_actual = modulo;
            self.archivo = archivo;
            self.cuerpo(id, f);
        }
        self.mod_actual = String::new();
        self.archivo = raiz;
        self.sin_recursion();
        if opciones.exige_principal {
            match self.funciones.get(PRINCIPAL) {
                None => self.error_con(
                    Lugar::nuevo(1, 1),
                    format!("falta la funcion `{PRINCIPAL}`, donde empieza el programa"),
                    format!("escribe `funcion {PRINCIPAL}() {{ ... }}`"),
                ),
                Some(&id) => {
                    let f = self.firmas[id].clone();
                    let lugar = self.modulo.funciones[id].lugar;
                    if !f.parametros.is_empty() || f.retorno != Tipo::Vacio {
                        self.error(
                            lugar,
                            format!("`{PRINCIPAL}` no puede tener parametros ni devolver un valor"),
                        );
                    }
                }
            }
        }
    }

    fn disponer_estructura(&mut self, indice: usize, d: &DefEstructura) {
        let mut campos = Vec::new();
        let mut siguiente = 0u64;
        let mut maximo = 0u64;
        let mut vistos = BTreeSet::new();
        for c in &d.campos {
            if !vistos.insert(c.nombre.clone()) {
                self.error(c.lugar, format!("el campo `{}` esta repetido", c.nombre));
                continue;
            }
            let tipo = self.resolver_tipo(&c.tipo);
            if let Tipo::Estructura(j) = tipo {
                if j == indice {
                    self.error(
                        c.lugar,
                        format!("la estructura `{}` se contiene a si misma", d.nombre),
                    );
                    continue;
                }
                if j > indice {
                    self.error_con(
                        c.lugar,
                        format!(
                            "la estructura `{}` se usa antes de declararse",
                            self.modulo.estructuras[j].nombre
                        ),
                        "declara primero la estructura que va dentro".to_string(),
                    );
                    continue;
                }
            }
            let tam = self.palabras_de(&tipo);
            let desplazamiento = match &c.desplazamiento {
                Some(e) => match self.constante_entera(e, "el desplazamiento de un campo") {
                    Some(v) => v as u64,
                    None => continue,
                },
                None => siguiente,
            };
            siguiente = desplazamiento + tam;
            maximo = maximo.max(siguiente);
            campos.push(hir::CampoH {
                nombre: c.nombre.clone(),
                tipo,
                desplazamiento,
            });
        }
        // Dos campos no pueden solaparse (RF-SGT-007).
        for (i, a) in campos.iter().enumerate() {
            for b in campos.iter().skip(i + 1) {
                let (ta, tb) = (self.palabras_de(&a.tipo), self.palabras_de(&b.tipo));
                if a.desplazamiento < b.desplazamiento + tb
                    && b.desplazamiento < a.desplazamiento + ta
                {
                    self.error(
                        d.lugar,
                        format!(
                            "los campos `{}` y `{}` de `{}` se solapan",
                            a.nombre, b.nombre, d.nombre
                        ),
                    );
                }
            }
        }
        self.modulo.estructuras[indice].campos = campos;
        self.modulo.estructuras[indice].tamano = maximo;
    }

    fn def_const(&mut self, c: &DefConst) {
        let clave = Self::nombre_en(&self.mod_actual, &c.nombre);
        if self.consts.contains_key(&clave)
            || self.globales.contains_key(&clave)
            || self
                .alias
                .contains_key(&(self.mod_actual.clone(), c.nombre.clone()))
        {
            self.error(c.lugar, format!("`{}` ya esta declarado", c.nombre));
            return;
        }
        let tipo = self.resolver_tipo(&c.tipo);
        if tipo.es_error() {
            return;
        }
        if !tipo.es_escalar() || matches!(tipo, Tipo::Puntero(_)) {
            self.error(
                c.lugar,
                "una constante solo puede ser un entero o un booleano",
            );
            return;
        }
        let v = self.expr(&c.valor, Some(&tipo));
        if v.tipo.es_error() {
            return;
        }
        if v.tipo != tipo {
            self.error_tipos(
                c.valor.lugar,
                format!(
                    "la constante `{}` es {} pero el valor es {}",
                    c.nombre, tipo, v.tipo
                ),
                &v.tipo,
                &tipo,
            );
            return;
        }
        match v.clase {
            CE::Const(valor) => {
                self.consts.insert(clave, (tipo, valor));
            }
            _ => self.error_con(
                c.valor.lugar,
                format!(
                    "el valor de la constante `{}` no se conoce al compilar",
                    c.nombre
                ),
                "usa solo numeros y otras constantes".to_string(),
            ),
        }
    }

    fn def_global(&mut self, g: &DefGlobal) {
        let clave = Self::nombre_en(&self.mod_actual, &g.nombre);
        if self.consts.contains_key(&clave)
            || self.globales.contains_key(&clave)
            || self
                .alias
                .contains_key(&(self.mod_actual.clone(), g.nombre.clone()))
        {
            self.error(g.lugar, format!("`{}` ya esta declarado", g.nombre));
            return;
        }
        let tipo = self.resolver_tipo(&g.tipo);
        let tam = self.palabras_de(&tipo);
        let direccion = g
            .direccion
            .as_ref()
            .and_then(|e| self.constante_entera(e, "la direccion de una variable"))
            .map(|v| v as u64);
        if let Some(d) = direccion {
            self.comprobar_ubicacion(&g.nombre, d, tam, g.lugar, g.inicial.is_some());
        }
        let mut datos = Vec::new();
        let texto = match (&g.inicial, &tipo) {
            (
                Some(Expr {
                    clase: ClaseExpr::Cadena(t),
                    lugar,
                }),
                Tipo::Arreglo(..),
            ) => Some((t, *lugar)),
            _ => None,
        };
        if let Some((t, lugar)) = texto {
            datos = self.texto_inicial(&g.nombre, &tipo, t, lugar);
        }
        let inicial = g
            .inicial
            .as_ref()
            .filter(|_| texto.is_none())
            .and_then(|e| {
                if !tipo.es_escalar() {
                    self.error(
                        e.lugar,
                        "solo una variable escalar puede tener valor inicial",
                    );
                    return None;
                }
                let v = self.expr(e, Some(&tipo));
                if v.tipo.es_error() {
                    return None;
                }
                if v.tipo != tipo {
                    self.error_tipos(
                        e.lugar,
                        format!(
                            "`{}` es {} pero el valor inicial es {}",
                            g.nombre, tipo, v.tipo
                        ),
                        &v.tipo,
                        &tipo,
                    );
                    return None;
                }
                if !matches!(v.clase, CE::Const(_)) {
                    self.error(
                        e.lugar,
                        "el valor inicial de una variable global debe ser constante",
                    );
                    return None;
                }
                Some(v)
            });
        let id = self.modulo.globales.len();
        self.globales.insert(clave.clone(), id);
        self.modulo.globales.push(hir::Variable {
            nombre: clave,
            tipo,
            lugar: g.lugar,
            direccion,
            inicial,
            datos,
        });
    }

    /// El texto con que se inicia un arreglo de enteros: un caracter por elemento y, tras el
    /// ultimo, un cero que lo termina (RF-SGT-041).
    fn texto_inicial(&mut self, nombre: &str, tipo: &Tipo, texto: &str, lugar: Lugar) -> Vec<i128> {
        let Tipo::Arreglo(elemento, n) = tipo else {
            return Vec::new();
        };
        let Tipo::Entero { bits, signo } = **elemento else {
            self.error(
                lugar,
                format!(
                    "un texto solo puede iniciar un arreglo de enteros, y `{nombre}` es {tipo}"
                ),
            );
            return Vec::new();
        };
        let codigos: Vec<i128> = texto.chars().map(|c| i128::from(u32::from(c))).collect();
        if codigos.len() as u64 + 1 > *n {
            self.error_con(
                lugar,
                format!(
                    "el texto tiene {} caracteres y `{nombre}` solo tiene {n} elementos",
                    codigos.len()
                ),
                "deja un elemento mas para el cero final".to_string(),
            );
            return Vec::new();
        }
        let (min, max) = crate::tipos::rango(bits, signo);
        if let Some(c) = codigos.iter().find(|c| **c < min || **c > max) {
            self.error(
                lugar,
                format!("el caracter con codigo {c} no cabe en {elemento}, el tipo de los elementos de `{nombre}`"),
            );
            return Vec::new();
        }
        codigos
    }

    /// RF-SGT-017 y RF-SGT-028: una variable ubicada cae dentro de una region y no en una de solo
    /// lectura si va a escribirse.
    fn comprobar_ubicacion(
        &mut self,
        nombre: &str,
        dir: u64,
        tam: u64,
        lugar: Lugar,
        escribe: bool,
    ) {
        let fin = dir + tam.max(1) - 1;
        let region = self.destino.region_de(dir).cloned();
        match region {
            None => self.error(
                lugar,
                format!(
                    "`{nombre}` se ubica en {dir:#X}, que no pertenece a ninguna region de `{}`",
                    self.destino.nombre
                ),
            ),
            Some(r) if fin > r.hasta => self.error(
                lugar,
                format!(
                    "`{nombre}` ocupa {dir:#X}..{fin:#X} y se sale de la region `{}` ({:#X}..{:#X})",
                    r.nombre, r.desde, r.hasta
                ),
            ),
            Some(r) if escribe && !r.escritura => self.error(
                lugar,
                format!(
                    "`{nombre}` se escribe pero esta en la region `{}`, que es de solo lectura en `{}`",
                    r.nombre, self.destino.nombre
                ),
            ),
            Some(_) => {}
        }
    }

    fn firma(&mut self, f: &DefFuncion) -> Option<hir::IdFuncion> {
        let clave = Self::nombre_en(&self.mod_actual, &f.nombre);
        if self.funciones.contains_key(&clave)
            || self
                .alias
                .contains_key(&(self.mod_actual.clone(), f.nombre.clone()))
        {
            self.error(
                f.lugar,
                format!("la funcion `{}` ya esta declarada", f.nombre),
            );
            return None;
        }
        let mut manejador = None;
        let mut presupuesto = None;
        for a in &f.atributos {
            if a.nombre == "presupuesto" {
                if a.argumentos.len() != 1 {
                    self.error(
                        a.lugar,
                        "`@presupuesto` lleva un solo argumento: los ciclos",
                    );
                } else if let Some(n) = self.constante_entera(&a.argumentos[0], "el presupuesto") {
                    if n <= 0 {
                        self.error(a.lugar, "el presupuesto debe ser de al menos un ciclo");
                    } else {
                        presupuesto = Some(n as u64);
                    }
                }
                continue;
            }
            if a.nombre != "interrupcion" {
                self.error_con(
                    a.lugar,
                    format!("el atributo `@{}` no existe", a.nombre),
                    "los atributos de funcion son `@interrupcion(linea)` y `@presupuesto(ciclos)`"
                        .to_string(),
                );
                continue;
            }
            if a.argumentos.len() != 1 {
                self.error(a.lugar, "`@interrupcion` lleva un solo argumento: la linea");
                continue;
            }
            if let Some(l) = self.constante_entera(&a.argumentos[0], "la linea de interrupcion") {
                let l = l as u32;
                if !self.destino.lineas.contains(&l) {
                    let declaradas: Vec<String> =
                        self.destino.lineas.iter().map(|x| x.to_string()).collect();
                    self.error(
                        a.lugar,
                        format!(
                            "la linea de interrupcion {l} no tiene vector declarado en `{}` (lineas con vector: {})",
                            self.destino.nombre,
                            if declaradas.is_empty() { "ninguna".to_string() } else { declaradas.join(", ") }
                        ),
                    );
                } else {
                    manejador = Some(l);
                }
            }
        }
        let parametros: Vec<Tipo> = f
            .parametros
            .iter()
            .map(|p| {
                let t = self.resolver_tipo(&p.tipo);
                if !t.es_escalar() && !t.es_error() {
                    self.error(
                        p.lugar,
                        "un parametro solo puede ser un entero, un booleano o un puntero",
                    );
                    Tipo::Error
                } else {
                    t
                }
            })
            .collect();
        let retorno = match &f.retorno {
            Some(t) => {
                let t = self.resolver_tipo(t);
                if !t.es_escalar() && !t.es_error() {
                    self.error(
                        f.lugar,
                        "una funcion solo puede devolver un entero, un booleano o un puntero",
                    );
                    Tipo::Error
                } else {
                    t
                }
            }
            None => Tipo::Vacio,
        };
        if manejador.is_some() && (!parametros.is_empty() || retorno != Tipo::Vacio) {
            self.error(
                f.lugar,
                "un manejador de interrupcion no puede tener parametros ni devolver un valor",
            );
        }
        let id = self.modulo.funciones.len();
        self.funciones.insert(clave.clone(), id);
        self.firmas.push(Firma {
            parametros,
            retorno: retorno.clone(),
            manejador: manejador.is_some(),
        });
        self.modulo.funciones.push(hir::Funcion {
            nombre: clave,
            base: f.nombre.clone(),
            raiz: 0,
            lugar: f.lugar,
            parametros: Vec::new(),
            locales: Vec::new(),
            retorno,
            cuerpo: Vec::new(),
            interrupcion: manejador,
            presupuesto,
        });
        Some(id)
    }

    fn cuerpo(&mut self, id: hir::IdFuncion, f: &DefFuncion) {
        self.locales = Vec::new();
        self.ambitos = vec![HashMap::new()];
        self.bucles = 0;
        self.retorno = self.firmas[id].retorno.clone();
        let mut parametros = Vec::new();
        for (p, t) in f.parametros.iter().zip(self.firmas[id].parametros.clone()) {
            if self.ambitos[0].contains_key(&p.nombre) {
                self.error(
                    p.lugar,
                    format!("el parametro `{}` esta repetido", p.nombre),
                );
                continue;
            }
            let l = self.nueva_local(&p.nombre, t, p.lugar);
            parametros.push(l);
        }
        let cuerpo = self.bloque(&f.cuerpo);
        if self.retorno != Tipo::Vacio
            && !self.retorna_siempre(&f.cuerpo)
            && !self.retorno.es_error()
        {
            self.error_con(
                f.lugar,
                format!(
                    "la funcion `{}` puede terminar sin devolver un valor",
                    f.nombre
                ),
                "termina todos los caminos con `devolver valor;`".to_string(),
            );
        }
        let fun = &mut self.modulo.funciones[id];
        fun.parametros = parametros;
        fun.locales = std::mem::take(&mut self.locales);
        fun.cuerpo = cuerpo;
    }

    fn nueva_local(&mut self, nombre: &str, tipo: Tipo, lugar: Lugar) -> hir::IdLocal {
        let id = self.locales.len();
        self.locales.push(hir::Variable {
            nombre: nombre.to_string(),
            tipo,
            lugar,
            direccion: None,
            inicial: None,
            datos: Vec::new(),
        });
        if let Some(a) = self.ambitos.last_mut() {
            a.insert(nombre.to_string(), id);
        }
        id
    }

    /// Si todos los caminos del bloque terminan en `devolver` (o en un bucle sin salida).
    fn retorna_siempre(&self, b: &Bloque) -> bool {
        b.sentencias
            .last()
            .is_some_and(|s| self.sentencia_retorna(s))
    }

    fn sentencia_retorna(&self, s: &Sentencia) -> bool {
        match &s.clase {
            ClaseSentencia::Devolver(_) => true,
            ClaseSentencia::Si { entonces, sino, .. } => {
                self.retorna_siempre(entonces)
                    && sino.as_ref().is_some_and(|b| self.retorna_siempre(b))
            }
            ClaseSentencia::Bucle(cuerpo) => !contiene_romper(cuerpo),
            ClaseSentencia::Segun { brazos, otro, .. } => {
                otro.as_ref().is_some_and(|b| self.retorna_siempre(b))
                    && brazos.iter().all(|(_, b)| self.retorna_siempre(b))
            }
            _ => false,
        }
    }

    /// Sin recursion: las variables locales se reparten de forma estatica (decision D-059).
    fn sin_recursion(&mut self) {
        let n = self.modulo.funciones.len();
        let mut destinos: Vec<Vec<(hir::IdFuncion, Lugar)>> = vec![Vec::new(); n];
        for (origen, f) in self.modulo.funciones.iter().enumerate() {
            let mut llamadas = Vec::new();
            for s in &f.cuerpo {
                recoger_llamadas_sent(s, &mut llamadas);
            }
            destinos[origen] = llamadas;
        }
        // Una funcion es recursiva si puede alcanzarse a si misma.
        let mut avisadas = BTreeSet::new();
        for inicio in 0..n {
            let mut visitados = vec![false; n];
            let mut pila: Vec<(hir::IdFuncion, Lugar)> = destinos[inicio].clone();
            let mut lugar_inicial = None;
            while let Some((f, lugar)) = pila.pop() {
                if lugar_inicial.is_none() {
                    lugar_inicial = Some(lugar);
                }
                if f == inicio {
                    if avisadas.insert(inicio) {
                        let nombre = self.modulo.funciones[inicio].nombre.clone();
                        self.error_con(
                            lugar,
                            format!("la funcion `{nombre}` se llama a si misma, directa o indirectamente"),
                            "esta version reparte las variables locales de forma estatica y no admite recursion; usa un bucle".to_string(),
                        );
                    }
                    break;
                }
                if !visitados[f] {
                    visitados[f] = true;
                    pila.extend(destinos[f].iter().copied());
                }
            }
        }
        let _ = &self.llamadas;
    }

    // ------------------------------------------------------------------
    // Sentencias
    // ------------------------------------------------------------------

    fn bloque(&mut self, b: &Bloque) -> Vec<hir::Sent> {
        self.ambitos.push(HashMap::new());
        let v = b
            .sentencias
            .iter()
            .filter_map(|s| self.sentencia(s))
            .collect();
        self.ambitos.pop();
        v
    }

    fn buscar_local(&self, nombre: &str) -> Option<hir::IdLocal> {
        self.ambitos
            .iter()
            .rev()
            .find_map(|a| a.get(nombre).copied())
    }

    fn sentencia(&mut self, s: &Sentencia) -> Option<hir::Sent> {
        let lugar = s.lugar;
        let clase = match &s.clase {
            ClaseSentencia::Var {
                nombre,
                tipo,
                inicial,
            } => {
                if self.ambitos.last().is_some_and(|a| a.contains_key(nombre)) {
                    self.error(
                        lugar,
                        format!("`{nombre}` ya esta declarada en este bloque"),
                    );
                    return None;
                }
                let t = tipo.as_ref().map(|t| self.resolver_tipo(t));
                let valor = inicial.as_ref().map(|e| self.expr(e, t.as_ref()));
                let tipo_final = match (&t, &valor) {
                    (Some(t), _) => t.clone(),
                    (None, Some(v)) => self.por_omision(&v.tipo),
                    (None, None) => Tipo::Error,
                };
                // El valor inicial de un literal sin tipo se vuelve a tipar con el tipo elegido.
                let valor = match (&valor, &t) {
                    (Some(v), None) if v.tipo != tipo_final && !v.tipo.es_error() => {
                        let e = inicial.as_ref().unwrap();
                        Some(self.expr(e, Some(&tipo_final)))
                    }
                    _ => valor,
                };
                if let Some(v) = &valor {
                    if !v.tipo.es_error() && !tipo_final.es_error() && v.tipo != tipo_final {
                        self.error_tipos(
                            inicial.as_ref().unwrap().lugar,
                            format!("`{nombre}` es {tipo_final} pero el valor es {}", v.tipo),
                            &v.tipo,
                            &tipo_final,
                        );
                    }
                }
                if !tipo_final.es_error() && inicial.is_some() && !tipo_final.es_escalar() {
                    self.error(lugar, "solo una variable escalar puede tener valor inicial");
                }
                if tipo_final == Tipo::Vacio {
                    self.error(lugar, format!("`{nombre}` no puede ser de tipo vacio"));
                }
                let id = self.nueva_local(nombre, tipo_final.clone(), lugar);
                // Sin valor inicial vale cero (decision D-060): se escribe al ejecutar la declaracion.
                let destino = Sitio {
                    tipo: tipo_final.clone(),
                    clase: ClaseSitio::Variable(Ref::Local(id)),
                };
                match valor {
                    Some(v) => ClaseSent::Asignar(destino, v),
                    None if tipo_final.es_escalar() => ClaseSent::Asignar(
                        destino,
                        hir::Expr {
                            lugar,
                            tipo: tipo_final,
                            clase: CE::Const(0),
                        },
                    ),
                    None => return None,
                }
            }
            ClaseSentencia::Asignar { destino, op, valor } => {
                let sitio = self.sitio(destino)?;
                if sitio.tipo.es_error() {
                    // Se analiza igual el valor para no perder sus errores.
                    self.expr(valor, None);
                    return None;
                }
                if !sitio.tipo.es_escalar() {
                    self.error(
                        lugar,
                        format!(
                            "no se puede asignar un valor de tipo {} de una vez",
                            sitio.tipo
                        ),
                    );
                    return None;
                }
                self.comprobar_escritura(&sitio, destino.lugar);
                let nuevo = match op {
                    None => self.expr(valor, Some(&sitio.tipo)),
                    Some(op) => {
                        let actual = hir::Expr {
                            lugar: destino.lugar,
                            tipo: sitio.tipo.clone(),
                            clase: CE::Leer(Box::new(sitio.clone())),
                        };
                        let derecha = self.expr(valor, Some(&sitio.tipo));
                        self.binario_tipado(*op, actual, derecha, lugar)
                    }
                };
                if !nuevo.tipo.es_error() && nuevo.tipo != sitio.tipo {
                    self.error_tipos(
                        valor.lugar,
                        format!(
                            "se asigna un valor de tipo {} a algo de tipo {}",
                            nuevo.tipo, sitio.tipo
                        ),
                        &nuevo.tipo,
                        &sitio.tipo,
                    );
                    return None;
                }
                if nuevo.tipo.es_error() {
                    return None;
                }
                ClaseSent::Asignar(sitio, nuevo)
            }
            ClaseSentencia::Si {
                condicion,
                entonces,
                sino,
            } => {
                let c = self.condicion(condicion);
                let e = self.bloque(entonces);
                let n = sino.as_ref().map(|b| self.bloque(b)).unwrap_or_default();
                ClaseSent::Si(c, e, n)
            }
            ClaseSentencia::Mientras { condicion, cuerpo } => {
                let c = self.condicion(condicion);
                self.bucles += 1;
                let b = self.bloque(cuerpo);
                self.bucles -= 1;
                ClaseSent::Mientras(c, b)
            }
            ClaseSentencia::Bucle(cuerpo) => {
                self.bucles += 1;
                let b = self.bloque(cuerpo);
                self.bucles -= 1;
                ClaseSent::Bucle(b)
            }
            ClaseSentencia::Romper => {
                if self.bucles == 0 {
                    self.error_con(
                        lugar,
                        "`romper` solo vale dentro de un bucle",
                        "ponlo dentro de un `mientras` o un `bucle`".to_string(),
                    );
                    return None;
                }
                ClaseSent::Romper
            }
            ClaseSentencia::Continuar => {
                if self.bucles == 0 {
                    self.error_con(
                        lugar,
                        "`continuar` solo vale dentro de un bucle",
                        "ponlo dentro de un `mientras` o un `bucle`".to_string(),
                    );
                    return None;
                }
                ClaseSent::Continuar
            }
            ClaseSentencia::Devolver(valor) => {
                let esperado = self.retorno.clone();
                match (valor, &esperado) {
                    (None, Tipo::Vacio) => ClaseSent::Devolver(None),
                    (None, t) if !t.es_error() => {
                        self.error_con(
                            lugar,
                            format!("la funcion devuelve {t} y falta el valor"),
                            "escribe `devolver valor;`".to_string(),
                        );
                        return None;
                    }
                    (Some(e), Tipo::Vacio) => {
                        self.error_con(
                            e.lugar,
                            "esta funcion no devuelve ningun valor",
                            "quita el valor o declara el tipo de retorno con `-> tipo`".to_string(),
                        );
                        return None;
                    }
                    (Some(e), t) => {
                        let v = self.expr(e, Some(t));
                        if !v.tipo.es_error() && !t.es_error() && &v.tipo != t {
                            self.error_tipos(
                                e.lugar,
                                format!("la funcion devuelve {t} pero el valor es {}", v.tipo),
                                &v.tipo,
                                t,
                            );
                            return None;
                        }
                        ClaseSent::Devolver(Some(v))
                    }
                    (None, _) => return None,
                }
            }
            ClaseSentencia::Segun {
                valor,
                brazos,
                otro,
            } => return self.segun(lugar, valor, brazos, otro),
            ClaseSentencia::Asm(texto) => self.asm(lugar, texto)?,
            ClaseSentencia::Expr(e) => {
                if let ClaseExpr::Llamada(n, args) = &e.clase {
                    if n == "salida" {
                        return self.salida(lugar, args);
                    }
                }
                let h = self.expr(e, None);
                if h.tipo.es_error() {
                    return None;
                }
                if !matches!(h.clase, CE::Llamada(..)) {
                    self.error_con(
                        e.lugar,
                        "esta expresion no hace nada",
                        "una sentencia debe ser una llamada, una asignacion o una orden"
                            .to_string(),
                    );
                    return None;
                }
                ClaseSent::Expr(h)
            }
        };
        Some(hir::Sent { lugar, clase })
    }

    fn por_omision(&self, t: &Tipo) -> Tipo {
        t.clone()
    }

    fn condicion(&mut self, e: &Expr) -> hir::Expr {
        let c = self.expr(e, Some(&Tipo::Bool));
        if !c.tipo.es_error() && c.tipo != Tipo::Bool {
            self.error_con(
                e.lugar,
                format!("la condicion es {} y debe ser bool", c.tipo),
                "compara con `!= 0` si quieres usar un entero".to_string(),
            );
            return hir::Expr {
                tipo: Tipo::Error,
                ..c
            };
        }
        c
    }

    fn segun(
        &mut self,
        lugar: Lugar,
        valor: &Expr,
        brazos: &[(Expr, Bloque)],
        otro: &Option<Bloque>,
    ) -> Option<hir::Sent> {
        let v = self.expr(valor, None);
        if v.tipo.es_error() {
            return None;
        }
        if !v.tipo.es_entero() {
            self.error(
                valor.lugar,
                format!("`segun` necesita un entero y esto es {}", v.tipo),
            );
            return None;
        }
        let mut vistos = BTreeSet::new();
        let mut hbrazos = Vec::new();
        for (e, b) in brazos {
            let c = self.expr(e, Some(&v.tipo));
            let cuerpo = self.bloque(b);
            if c.tipo.es_error() {
                continue;
            }
            match c.clase {
                CE::Const(x) if c.tipo == v.tipo => {
                    if !vistos.insert(x) {
                        self.error(
                            e.lugar,
                            format!("el valor {x} ya tiene un brazo en este `segun`"),
                        );
                    } else {
                        hbrazos.push((x, cuerpo));
                    }
                }
                _ => self.error(
                    e.lugar,
                    format!("el valor de un brazo debe ser una constante {}", v.tipo),
                ),
            }
        }
        let otro = otro.as_ref().map(|b| self.bloque(b)).unwrap_or_default();
        Some(hir::Sent {
            lugar,
            clase: ClaseSent::Segun {
                valor: v,
                brazos: hbrazos,
                otro,
            },
        })
    }

    fn asm(&mut self, lugar: Lugar, texto: &str) -> Option<ClaseSent> {
        let mut refs = Vec::new();
        let mut resto = texto;
        while let Some(i) = resto.find('{') {
            let fin = resto[i..].find('}').map(|j| i + j);
            let Some(fin) = fin else {
                self.error(lugar, "falta `}` en un `{nombre}` del ensamblador en linea");
                return None;
            };
            let nombre = resto[i + 1..fin].trim().to_string();
            let r = if let Some(l) = self.buscar_local(&nombre) {
                Ref::Local(l)
            } else if let Some(&g) = self.globales.get(&self.clave(&nombre)) {
                Ref::Global(g)
            } else {
                self.error(
                    lugar,
                    format!("el ensamblador en linea cita `{nombre}`, que no es una variable"),
                );
                return None;
            };
            refs.push((nombre, r));
            resto = &resto[fin + 1..];
        }
        Some(ClaseSent::Asm(texto.to_string(), refs))
    }

    fn salida(&mut self, lugar: Lugar, args: &[Expr]) -> Option<hir::Sent> {
        if args.len() != 2 {
            self.error(lugar, "`salida` lleva dos argumentos: el puerto y el valor");
            return None;
        }
        let puerto = self.puerto(&args[0])?;
        let valor = self.expr(&args[1], Some(&Tipo::u(self.destino.palabra)));
        if valor.tipo.es_error() {
            return None;
        }
        if valor.tipo != Tipo::u(self.destino.palabra) {
            self.error(
                args[1].lugar,
                format!(
                    "`salida` escribe una palabra de {} bits (u{}) y esto es {}",
                    self.destino.palabra, self.destino.palabra, valor.tipo
                ),
            );
            return None;
        }
        Some(hir::Sent {
            lugar,
            clase: ClaseSent::Salida(puerto, valor),
        })
    }

    /// RF-SGT-014 y RF-SGT-029: el puerto es una constante que algun dispositivo declara.
    fn puerto(&mut self, e: &Expr) -> Option<hir::Expr> {
        let p = self.expr(e, Some(&Tipo::u(self.destino.palabra)));
        if p.tipo.es_error() {
            return None;
        }
        let CE::Const(n) = p.clase else {
            self.error(
                e.lugar,
                "el puerto debe ser una constante conocida al compilar",
            );
            return None;
        };
        if !self.destino.puertos.contains(&(n as u64)) {
            let declarados: Vec<String> = self
                .destino
                .puertos
                .iter()
                .map(|x| format!("{x:#X}"))
                .collect();
            let detalle = if declarados.is_empty() {
                format!("`{}` no declara ningun puerto", self.destino.nombre)
            } else {
                format!(
                    "`{}` solo declara {}",
                    self.destino.nombre,
                    declarados.join(", ")
                )
            };
            self.error(
                e.lugar,
                format!("el puerto {n:#X} no esta declarado: {detalle}"),
            );
            return None;
        }
        Some(p)
    }

    // ------------------------------------------------------------------
    // Lugares
    // ------------------------------------------------------------------

    /// Analiza una expresion que designa un lugar de memoria o un registro.
    fn sitio(&mut self, e: &Expr) -> Option<Sitio> {
        match &e.clase {
            ClaseExpr::Nombre(n) => {
                if let Some(l) = self.buscar_local(n) {
                    return Some(Sitio {
                        tipo: self.locales[l].tipo.clone(),
                        clase: ClaseSitio::Variable(Ref::Local(l)),
                    });
                }
                if let Some(&g) = self.globales.get(&self.clave(n)) {
                    return Some(Sitio {
                        tipo: self.modulo.globales[g].tipo.clone(),
                        clase: ClaseSitio::Variable(Ref::Global(g)),
                    });
                }
                if self.consts.contains_key(&self.clave(n)) {
                    self.error(
                        e.lugar,
                        format!("`{n}` es una constante y no se puede modificar"),
                    );
                } else {
                    self.nombre_desconocido(n, e.lugar);
                }
                None
            }
            ClaseExpr::Indice(base, i) => {
                let idx = self.expr(i, Some(&Tipo::u(self.destino.palabra)));
                let b = self.expr_o_sitio(base)?;
                if idx.tipo.es_error() {
                    return None;
                }
                if !idx.tipo.es_entero() {
                    let m = format!("el indice debe ser un entero y esto es {}", idx.tipo);
                    let t = Tipo::u(self.destino.palabra);
                    self.error_tipos(i.lugar, m, &idx.tipo, &t);
                    return None;
                }
                match b {
                    Base::Sitio(s) => match s.tipo.clone() {
                        Tipo::Arreglo(el, n) => {
                            if let CE::Const(v) = idx.clase {
                                if v < 0 || v as u64 >= n {
                                    self.error(
                                        i.lugar,
                                        format!(
                                            "el indice {v} se sale de un arreglo de {n} elementos"
                                        ),
                                    );
                                    return None;
                                }
                            }
                            let tam = self.palabras_de(&el);
                            Some(Sitio {
                                tipo: *el,
                                clase: ClaseSitio::Indice(Box::new(s), Box::new(idx), tam),
                            })
                        }
                        Tipo::Puntero(el) => Some(self.indice_de_puntero(
                            hir::Expr {
                                lugar: base.lugar,
                                tipo: Tipo::Puntero(el.clone()),
                                clase: CE::Leer(Box::new(s)),
                            },
                            *el,
                            idx,
                            e.lugar,
                        )),
                        t => {
                            self.error(base.lugar, format!("no se puede indexar algo de tipo {t}"));
                            None
                        }
                    },
                    Base::Valor(v) => match v.tipo.clone() {
                        Tipo::Puntero(el) => Some(self.indice_de_puntero(v, *el, idx, e.lugar)),
                        t => {
                            self.error(base.lugar, format!("no se puede indexar algo de tipo {t}"));
                            None
                        }
                    },
                }
            }
            ClaseExpr::Campo(base, campo) => {
                if let ClaseExpr::Nombre(n) = &base.clase {
                    if n == "reg"
                        && self.buscar_local(n).is_none()
                        && !self.globales.contains_key(&self.clave(n))
                    {
                        return self.registro(campo, e.lugar);
                    }
                }
                let b = self.sitio(base)?;
                match &b.tipo {
                    Tipo::Estructura(i) => {
                        let est = &self.modulo.estructuras[*i];
                        match est.campos.iter().find(|c| &c.nombre == campo) {
                            Some(c) => {
                                let (tipo, desplazamiento) = (c.tipo.clone(), c.desplazamiento);
                                Some(Sitio {
                                    tipo,
                                    clase: ClaseSitio::Campo(Box::new(b), desplazamiento),
                                })
                            }
                            None => {
                                let nombre = est.nombre.clone();
                                let campos: Vec<String> =
                                    est.campos.iter().map(|c| c.nombre.clone()).collect();
                                self.error(
                                    e.lugar,
                                    format!(
                                        "la estructura `{nombre}` no tiene el campo `{campo}` (tiene: {})",
                                        campos.join(", ")
                                    ),
                                );
                                None
                            }
                        }
                    }
                    Tipo::Error => None,
                    t => {
                        let t = t.clone();
                        self.error(e.lugar, format!("algo de tipo {t} no tiene campos"));
                        None
                    }
                }
            }
            ClaseExpr::Deref(p) => {
                let v = self.expr(p, None);
                match &v.tipo {
                    Tipo::Puntero(t) => {
                        let tipo = (**t).clone();
                        Some(Sitio {
                            tipo,
                            clase: ClaseSitio::Deref(Box::new(v)),
                        })
                    }
                    Tipo::Error => None,
                    t => {
                        let t = t.clone();
                        self.error(e.lugar, format!("`*` necesita un puntero y esto es {t}"));
                        None
                    }
                }
            }
            _ => {
                self.error(e.lugar, "a la izquierda de `=` debe haber una variable, un elemento, un campo o un registro");
                None
            }
        }
    }

    fn indice_de_puntero(&mut self, p: hir::Expr, el: Tipo, idx: hir::Expr, lugar: Lugar) -> Sitio {
        let tam = self.palabras_de(&el);
        let palabra = Tipo::u(self.destino.direccion);
        let _ = (&idx, &palabra);
        // p[i] = *(p + i * tam)
        let escala = hir::Expr {
            lugar,
            tipo: idx.tipo.clone(),
            clase: CE::Const(tam as i128),
        };
        let desplazado = if tam == 1 {
            idx
        } else {
            hir::Expr {
                lugar,
                tipo: idx.tipo.clone(),
                clase: CE::Binario(OpBinario::Mul, Box::new(idx), Box::new(escala)),
            }
        };
        let tipo_p = p.tipo.clone();
        let suma = hir::Expr {
            lugar,
            tipo: tipo_p,
            clase: CE::Binario(OpBinario::Suma, Box::new(p), Box::new(desplazado)),
        };
        Sitio {
            tipo: el,
            clase: ClaseSitio::Deref(Box::new(suma)),
        }
    }

    fn registro(&mut self, nombre: &str, lugar: Lugar) -> Option<Sitio> {
        match self.destino.registros.iter().find(|(n, _)| n == nombre) {
            Some((_, bits)) if self.destino.anchos_de_entero().contains(bits) => Some(Sitio {
                tipo: Tipo::u(*bits),
                clase: ClaseSitio::Registro(nombre.to_string()),
            }),
            Some((_, bits)) => {
                let m = format!(
                    "el registro `{nombre}` tiene {bits} bits, que no es un ancho de entero de `{}`",
                    self.destino.nombre
                );
                self.error(lugar, m);
                None
            }
            None => {
                let todos: Vec<&str> = self
                    .destino
                    .registros
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect();
                let m = format!(
                    "`{}` no tiene el registro `{nombre}` (tiene: {})",
                    self.destino.nombre,
                    todos.join(", ")
                );
                self.error(lugar, m);
                None
            }
        }
    }

    /// RF-SGT-028: no se escribe en una region de solo lectura.
    fn comprobar_escritura(&mut self, s: &Sitio, lugar: Lugar) {
        match &s.clase {
            ClaseSitio::Variable(Ref::Global(g)) => {
                let v = &self.modulo.globales[*g];
                if let Some(d) = v.direccion {
                    if let Some(r) = self.destino.region_de(d) {
                        if !r.escritura {
                            let m = format!(
                                "`{}` esta en la region `{}`, que es de solo lectura en `{}`",
                                v.nombre, r.nombre, self.destino.nombre
                            );
                            self.error(lugar, m);
                        }
                    }
                }
            }
            ClaseSitio::Deref(p) => {
                if let CE::Const(d) = p.clase {
                    if let Some(r) = self.destino.region_de(d as u64) {
                        if !r.escritura {
                            let m = format!(
                                "la direccion {d:#X} esta en la region `{}`, que es de solo lectura en `{}`",
                                r.nombre, self.destino.nombre
                            );
                            self.error(lugar, m);
                        }
                    }
                }
            }
            ClaseSitio::Indice(b, _, _) | ClaseSitio::Campo(b, _) => {
                self.comprobar_escritura(b, lugar)
            }
            _ => {}
        }
    }

    fn nombre_desconocido(&mut self, n: &str, lugar: Lugar) {
        let mut visibles: Vec<String> = self
            .ambitos
            .iter()
            .flat_map(|a| a.keys().cloned())
            .chain(self.globales.keys().cloned())
            .chain(self.consts.keys().cloned())
            .collect();
        visibles.sort();
        let cercano = visibles
            .iter()
            .filter(|v| distancia(v, n) <= 2)
            .min_by_key(|v| distancia(v, n));
        let d = Diagnostico::nuevo(&self.archivo, lugar, format!("no existe `{n}`"));
        self.diag.push(match cercano {
            Some(c) => d.con_sugerencia(format!("quiza querias decir `{c}`")),
            None => d,
        });
    }

    // ------------------------------------------------------------------
    // Expresiones
    // ------------------------------------------------------------------

    fn expr_o_sitio(&mut self, e: &Expr) -> Option<Base> {
        match &e.clase {
            ClaseExpr::Nombre(n)
                if self.buscar_local(n).is_some() || self.globales.contains_key(&self.clave(n)) =>
            {
                self.sitio(e).map(Base::Sitio)
            }
            ClaseExpr::Indice(..) | ClaseExpr::Campo(..) | ClaseExpr::Deref(_) => {
                self.sitio(e).map(Base::Sitio)
            }
            _ => {
                let v = self.expr(e, None);
                if v.tipo.es_error() {
                    None
                } else {
                    Some(Base::Valor(v))
                }
            }
        }
    }

    fn error_expr(&self, lugar: Lugar) -> hir::Expr {
        hir::Expr {
            lugar,
            tipo: Tipo::Error,
            clase: CE::Const(0),
        }
    }

    fn constante(&self, lugar: Lugar, tipo: Tipo, v: i128) -> hir::Expr {
        hir::Expr {
            lugar,
            tipo,
            clase: CE::Const(v),
        }
    }

    fn leer(&self, lugar: Lugar, s: Sitio) -> hir::Expr {
        hir::Expr {
            lugar,
            tipo: s.tipo.clone(),
            clase: CE::Leer(Box::new(s)),
        }
    }

    fn expr(&mut self, e: &Expr, esperado: Option<&Tipo>) -> hir::Expr {
        let lugar = e.lugar;
        match &e.clase {
            ClaseExpr::Entero(n) => self.literal(lugar, i128::from(*n), esperado),
            ClaseExpr::Bool(b) => self.constante(lugar, Tipo::Bool, i128::from(*b)),
            ClaseExpr::Cadena(_) => {
                self.error_con(
                    lugar,
                    "los textos todavia no tienen un tipo en SGT",
                    "usa un arreglo de enteros con los codigos de los caracteres".to_string(),
                );
                self.error_expr(lugar)
            }
            ClaseExpr::Nombre(n) => {
                if self.buscar_local(n).is_some() || self.globales.contains_key(&self.clave(n)) {
                    return match self.sitio(e) {
                        Some(s) => self.leer_valor(lugar, s),
                        None => self.error_expr(lugar),
                    };
                }
                if let Some((t, v)) = self.consts.get(&self.clave(n)).cloned() {
                    return self.constante(lugar, t, v);
                }
                if self.funciones.contains_key(&self.clave(n)) {
                    self.error(
                        lugar,
                        format!("`{n}` es una funcion; para llamarla escribe `{n}(...)`"),
                    );
                    return self.error_expr(lugar);
                }
                self.nombre_desconocido(n, lugar);
                self.error_expr(lugar)
            }
            ClaseExpr::Unario(op, a) => self.unario(lugar, *op, a, esperado),
            ClaseExpr::Binario(op, a, b) => self.binario(lugar, *op, a, b, esperado),
            ClaseExpr::Llamada(n, args) => self.llamada(lugar, n, args),
            ClaseExpr::LlamadaGenerica(n, ..) => {
                // El expansor de genericos las sustituye antes de llegar aqui.
                self.error_con(
                    lugar,
                    format!("la llamada generica a `{n}` no se expandio"),
                    "esto es un fallo del compilador; avisa de el".to_string(),
                );
                self.error_expr(lugar)
            }
            ClaseExpr::Indice(..) => match self.sitio(e) {
                Some(s) => self.leer_valor(lugar, s),
                None => self.error_expr(lugar),
            },
            ClaseExpr::Campo(base, campo) => {
                if let ClaseExpr::Nombre(n) = &base.clase {
                    if n == "bandera"
                        && self.buscar_local(n).is_none()
                        && !self.globales.contains_key(&self.clave(n))
                    {
                        if self.destino.banderas.iter().any(|b| b == campo) {
                            return hir::Expr {
                                lugar,
                                tipo: Tipo::Bool,
                                clase: CE::Bandera(campo.clone()),
                            };
                        }
                        let m = format!(
                            "`{}` no tiene la bandera `{campo}` (tiene: {})",
                            self.destino.nombre,
                            if self.destino.banderas.is_empty() {
                                "ninguna".to_string()
                            } else {
                                self.destino.banderas.join(", ")
                            }
                        );
                        self.error(lugar, m);
                        return self.error_expr(lugar);
                    }
                }
                match self.sitio(e) {
                    Some(s) => self.leer_valor(lugar, s),
                    None => self.error_expr(lugar),
                }
            }
            ClaseExpr::Deref(_) => match self.sitio(e) {
                Some(s) => self.leer_valor(lugar, s),
                None => self.error_expr(lugar),
            },
            ClaseExpr::DireccionDe(a) => match self.sitio(a) {
                Some(s) => {
                    if matches!(s.clase, ClaseSitio::Registro(_)) {
                        self.error(lugar, "un registro no tiene direccion");
                        return self.error_expr(lugar);
                    }
                    hir::Expr {
                        lugar,
                        tipo: Tipo::Puntero(Box::new(s.tipo.clone())),
                        clase: CE::Direccion(Box::new(s)),
                    }
                }
                None => self.error_expr(lugar),
            },
            ClaseExpr::Cast(a, t) => self.conversion(lugar, a, t),
        }
    }

    /// Leer un lugar como valor: solo los escalares se leen enteros.
    fn leer_valor(&mut self, lugar: Lugar, s: Sitio) -> hir::Expr {
        if s.tipo.es_error() {
            return self.error_expr(lugar);
        }
        if !s.tipo.es_escalar() {
            self.error_con(
                lugar,
                format!("no se puede usar de golpe un valor de tipo {}", s.tipo),
                "usa un elemento o un campo, o toma su direccion con `&`".to_string(),
            );
            return self.error_expr(lugar);
        }
        self.leer(lugar, s)
    }

    fn literal(&mut self, lugar: Lugar, v: i128, esperado: Option<&Tipo>) -> hir::Expr {
        let tipo = match esperado {
            Some(t @ Tipo::Entero { .. }) => t.clone(),
            Some(Tipo::Puntero(_)) | None | Some(Tipo::Bool) | Some(_) => {
                if matches!(esperado, Some(Tipo::Bool)) {
                    return self.constante(lugar, Tipo::u(self.destino.palabra), v);
                }
                Tipo::u(self.destino.palabra)
            }
        };
        if let Tipo::Entero { bits, signo } = tipo {
            let (min, max) = rango(bits, signo);
            if v > max || v < min {
                self.error_con(
                    lugar,
                    format!("{v} no cabe en {tipo}, que va de {min} a {max}"),
                    if v <= rango(bits * 2, false).1 && bits * 2 <= 64 {
                        format!("usa u{} o un valor menor", bits * 2)
                    } else {
                        "usa un valor menor".to_string()
                    },
                );
                return self.error_expr(lugar);
            }
        }
        self.constante(lugar, tipo, v)
    }

    fn unario(
        &mut self,
        lugar: Lugar,
        op: OpUnario,
        a: &Expr,
        esperado: Option<&Tipo>,
    ) -> hir::Expr {
        // `-128` con tipo i8: el literal es 128, que no cabe, asi que se trata el signo con el numero.
        if let (OpUnario::Negar, ClaseExpr::Entero(n)) = (op, &a.clase) {
            if let Some(t @ Tipo::Entero { signo: true, .. }) = esperado {
                return self.literal(lugar, -i128::from(*n), Some(t));
            }
        }
        let v = self.expr(
            a,
            if op == OpUnario::No {
                Some(&Tipo::Bool)
            } else {
                esperado
            },
        );
        if v.tipo.es_error() {
            return v;
        }
        let tipo_valido = match op {
            OpUnario::Negar => matches!(v.tipo, Tipo::Entero { signo: true, .. }),
            OpUnario::Complemento => v.tipo.es_entero(),
            OpUnario::No => v.tipo == Tipo::Bool,
        };
        if !tipo_valido {
            let (que, sug) = match op {
                OpUnario::Negar => (
                    "`-` necesita un entero con signo",
                    "para un entero sin signo usa `0 - x` o convierte con `como`",
                ),
                OpUnario::Complemento => ("`~` necesita un entero", "usa `!` con un bool"),
                OpUnario::No => (
                    "`!` necesita un bool",
                    "usa `~` para invertir los bits de un entero",
                ),
            };
            self.error_con(
                lugar,
                format!("{que} y esto es {}", v.tipo),
                sug.to_string(),
            );
            return self.error_expr(lugar);
        }
        if let CE::Const(x) = v.clase {
            let (bits, signo) = match v.tipo {
                Tipo::Entero { bits, signo } => (bits, signo),
                _ => (1, false),
            };
            return self.constante(lugar, v.tipo, valores::unario(op, x, bits, signo));
        }
        hir::Expr {
            lugar,
            tipo: v.tipo.clone(),
            clase: CE::Unario(op, Box::new(v)),
        }
    }

    fn es_literal(e: &Expr) -> bool {
        match &e.clase {
            ClaseExpr::Entero(_) => true,
            ClaseExpr::Unario(_, a) => Self::es_literal(a),
            ClaseExpr::Binario(_, a, b) => Self::es_literal(a) && Self::es_literal(b),
            _ => false,
        }
    }

    fn binario(
        &mut self,
        lugar: Lugar,
        op: OpBinario,
        a: &Expr,
        b: &Expr,
        esperado: Option<&Tipo>,
    ) -> hir::Expr {
        let (ha, hb);
        let contexto =
            if op.es_comparacion() || matches!(op, OpBinario::YLogico | OpBinario::OLogico) {
                None
            } else {
                esperado
            };
        if matches!(op, OpBinario::YLogico | OpBinario::OLogico) {
            ha = self.expr(a, Some(&Tipo::Bool));
            hb = self.expr(b, Some(&Tipo::Bool));
        } else if Self::es_literal(a) && !Self::es_literal(b) {
            hb = self.expr(b, contexto);
            ha = self.expr(a, Some(&hb.tipo));
        } else {
            ha = self.expr(a, contexto);
            let esp_b = match (&ha.tipo, op) {
                (Tipo::Puntero(_), OpBinario::Suma | OpBinario::Resta) => {
                    Some(Tipo::u(self.destino.direccion))
                }
                (t, _) => Some(t.clone()),
            };
            hb = self.expr(b, esp_b.as_ref());
        }
        self.binario_tipado(op, ha, hb, lugar)
    }

    fn binario_tipado(
        &mut self,
        op: OpBinario,
        a: hir::Expr,
        b: hir::Expr,
        lugar: Lugar,
    ) -> hir::Expr {
        if a.tipo.es_error() || b.tipo.es_error() {
            return self.error_expr(lugar);
        }
        let simbolo = op.simbolo();
        // Puntero con aritmetica explicita: `p + n` avanza n palabras (decision D-061).
        if let Tipo::Puntero(_) = a.tipo {
            if matches!(op, OpBinario::Suma | OpBinario::Resta) && b.tipo.es_entero() {
                return hir::Expr {
                    lugar,
                    tipo: a.tipo.clone(),
                    clase: CE::Binario(op, Box::new(a), Box::new(b)),
                };
            }
            if op.es_comparacion() && a.tipo == b.tipo {
                return hir::Expr {
                    lugar,
                    tipo: Tipo::Bool,
                    clase: CE::Binario(op, Box::new(a), Box::new(b)),
                };
            }
        }
        if a.tipo != b.tipo {
            self.error_con(
                lugar,
                format!("`{simbolo}` mezcla {} con {}", a.tipo, b.tipo),
                "convierte uno de los dos con `como`".to_string(),
            );
            return self.error_expr(lugar);
        }
        let t = a.tipo.clone();
        let logico = matches!(op, OpBinario::YLogico | OpBinario::OLogico);
        let valido = if logico {
            t == Tipo::Bool
        } else if op.es_comparacion() {
            t.es_entero() || t == Tipo::Bool && matches!(op, OpBinario::Igual | OpBinario::Distinto)
        } else {
            t.es_entero()
        };
        if !valido {
            self.error(lugar, format!("`{simbolo}` no se puede usar con {t}"));
            return self.error_expr(lugar);
        }
        let resultado = if op.es_comparacion() || logico {
            Tipo::Bool
        } else {
            t.clone()
        };
        if let (CE::Const(x), CE::Const(y)) = (&a.clase, &b.clase) {
            let (bits, signo) = match t {
                Tipo::Entero { bits, signo } => (bits, signo),
                _ => (1, false),
            };
            return match valores::binario(op, *x, *y, bits, signo) {
                Ok(v) => self.constante(lugar, resultado, v),
                Err(_) => {
                    self.error_con(
                        lugar,
                        "division entre cero",
                        "cambia el divisor o comprueba antes que no es cero".to_string(),
                    );
                    self.error_expr(lugar)
                }
            };
        }
        if matches!(op, OpBinario::Div | OpBinario::Mod) {
            if let CE::Const(0) = b.clase {
                self.error_con(
                    lugar,
                    "division entre cero",
                    "cambia el divisor o comprueba antes que no es cero".to_string(),
                );
                return self.error_expr(lugar);
            }
        }
        hir::Expr {
            lugar,
            tipo: resultado,
            clase: CE::Binario(op, Box::new(a), Box::new(b)),
        }
    }

    fn llamada(&mut self, lugar: Lugar, nombre: &str, args: &[Expr]) -> hir::Expr {
        if nombre == "entrada" {
            if args.len() != 1 {
                self.error(lugar, "`entrada` lleva un argumento: el puerto");
                return self.error_expr(lugar);
            }
            return match self.puerto(&args[0]) {
                Some(p) => hir::Expr {
                    lugar,
                    tipo: Tipo::u(self.destino.palabra),
                    clase: CE::Entrada(Box::new(p)),
                },
                None => self.error_expr(lugar),
            };
        }
        if nombre == "salida" {
            self.error_con(
                lugar,
                "`salida` no devuelve un valor",
                "escribela como sentencia: `salida(puerto, valor);`".to_string(),
            );
            return self.error_expr(lugar);
        }
        let Some(&id) = self.funciones.get(&self.clave(nombre)) else {
            self.nombre_desconocido(nombre, lugar);
            for a in args {
                self.expr(a, None);
            }
            return self.error_expr(lugar);
        };
        let firma = self.firmas[id].clone();
        if firma.manejador {
            self.error(
                lugar,
                format!("`{nombre}` es un manejador de interrupcion y no se llama a mano"),
            );
            return self.error_expr(lugar);
        }
        if args.len() != firma.parametros.len() {
            let tipos: Vec<String> = firma.parametros.iter().map(|t| t.to_string()).collect();
            self.error_con(
                lugar,
                format!(
                    "`{nombre}` recibe {} argumentos y se le pasan {}",
                    firma.parametros.len(),
                    args.len()
                ),
                format!("la firma es `{nombre}({})`", tipos.join(", ")),
            );
            for a in args {
                self.expr(a, None);
            }
            return self.error_expr(lugar);
        }
        let mut hargs = Vec::new();
        let mut ok = true;
        for (a, t) in args.iter().zip(&firma.parametros) {
            let h = self.expr(a, Some(t));
            if h.tipo.es_error() || t.es_error() {
                ok = false;
            } else if &h.tipo != t {
                self.error_tipos(
                    a.lugar,
                    format!("`{nombre}` espera {t} y se le pasa {}", h.tipo),
                    &h.tipo,
                    t,
                );
                ok = false;
            }
            hargs.push(h);
        }
        if !ok {
            return self.error_expr(lugar);
        }
        self.llamadas.push((id, lugar));
        hir::Expr {
            lugar,
            tipo: firma.retorno,
            clase: CE::Llamada(id, hargs),
        }
    }

    fn conversion(&mut self, lugar: Lugar, a: &Expr, t: &TipoAst) -> hir::Expr {
        let destino = self.resolver_tipo(t);
        // Un literal con `como` toma ese tipo directamente: `300 como u16`.
        let v = if Self::es_literal(a) && destino.es_entero() {
            self.expr(a, Some(&destino))
        } else {
            self.expr(a, None)
        };
        if v.tipo.es_error() || destino.es_error() {
            return self.error_expr(lugar);
        }
        let origen = v.tipo.clone();
        let permitido = match (&origen, &destino) {
            (Tipo::Entero { .. }, Tipo::Entero { .. }) => true,
            (Tipo::Bool, Tipo::Entero { .. }) => true,
            (Tipo::Puntero(_), Tipo::Entero { bits, .. }) => *bits == self.destino.direccion,
            (Tipo::Entero { bits, .. }, Tipo::Puntero(_)) => *bits == self.destino.direccion,
            (Tipo::Puntero(_), Tipo::Puntero(_)) => true,
            _ => false,
        };
        if !permitido {
            self.error_con(
                lugar,
                format!("no se puede convertir {origen} en {destino}"),
                if origen.es_entero() && destino == Tipo::Bool {
                    "compara con `!= 0`".to_string()
                } else {
                    "las conversiones validas son entre enteros, de bool a entero y entre punteros"
                        .to_string()
                },
            );
            return self.error_expr(lugar);
        }
        if let (CE::Const(x), Tipo::Entero { bits, signo }) = (&v.clase, &destino) {
            return self.constante(lugar, destino.clone(), envolver(*x, *bits, *signo));
        }
        // Un entero constante convertido en puntero es una direccion fija.
        if let (CE::Const(x), Tipo::Puntero(_)) = (&v.clase, &destino) {
            if v.tipo.es_entero() {
                return self.constante(lugar, destino, *x);
            }
        }
        hir::Expr {
            lugar,
            tipo: destino,
            clase: CE::Cast(Box::new(v)),
        }
    }
}

enum Base {
    Sitio(Sitio),
    Valor(hir::Expr),
}

impl Tipo {
    fn is_error_or_vacio(&self) -> bool {
        matches!(self, Tipo::Error | Tipo::Vacio)
    }
}

fn contiene_romper(b: &Bloque) -> bool {
    b.sentencias.iter().any(|s| match &s.clase {
        ClaseSentencia::Romper => true,
        ClaseSentencia::Si { entonces, sino, .. } => {
            contiene_romper(entonces) || sino.as_ref().is_some_and(contiene_romper)
        }
        ClaseSentencia::Segun { brazos, otro, .. } => {
            brazos.iter().any(|(_, b)| contiene_romper(b))
                || otro.as_ref().is_some_and(contiene_romper)
        }
        // Un `romper` dentro de otro bucle sale de ese bucle, no de este.
        _ => false,
    })
}

fn recoger_llamadas_sent(s: &hir::Sent, v: &mut Vec<(hir::IdFuncion, Lugar)>) {
    match &s.clase {
        ClaseSent::Asignar(sitio, e) => {
            recoger_llamadas_sitio(sitio, v);
            recoger_llamadas_expr(e, v);
        }
        ClaseSent::Si(c, a, b) => {
            recoger_llamadas_expr(c, v);
            a.iter().chain(b).for_each(|s| recoger_llamadas_sent(s, v));
        }
        ClaseSent::Mientras(c, b) => {
            recoger_llamadas_expr(c, v);
            b.iter().for_each(|s| recoger_llamadas_sent(s, v));
        }
        ClaseSent::Bucle(b) => b.iter().for_each(|s| recoger_llamadas_sent(s, v)),
        ClaseSent::Devolver(Some(e)) | ClaseSent::Expr(e) => recoger_llamadas_expr(e, v),
        ClaseSent::Segun {
            valor,
            brazos,
            otro,
        } => {
            recoger_llamadas_expr(valor, v);
            brazos
                .iter()
                .for_each(|(_, b)| b.iter().for_each(|s| recoger_llamadas_sent(s, v)));
            otro.iter().for_each(|s| recoger_llamadas_sent(s, v));
        }
        ClaseSent::Salida(a, b) => {
            recoger_llamadas_expr(a, v);
            recoger_llamadas_expr(b, v);
        }
        ClaseSent::Romper
        | ClaseSent::Continuar
        | ClaseSent::Devolver(None)
        | ClaseSent::Asm(..) => {}
    }
}

fn recoger_llamadas_sitio(s: &Sitio, v: &mut Vec<(hir::IdFuncion, Lugar)>) {
    match &s.clase {
        ClaseSitio::Indice(b, i, _) => {
            recoger_llamadas_sitio(b, v);
            recoger_llamadas_expr(i, v);
        }
        ClaseSitio::Campo(b, _) => recoger_llamadas_sitio(b, v),
        ClaseSitio::Deref(e) => recoger_llamadas_expr(e, v),
        ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => {}
    }
}

fn recoger_llamadas_expr(e: &hir::Expr, v: &mut Vec<(hir::IdFuncion, Lugar)>) {
    match &e.clase {
        CE::Llamada(f, args) => {
            v.push((*f, e.lugar));
            args.iter().for_each(|a| recoger_llamadas_expr(a, v));
        }
        CE::Leer(s) | CE::Direccion(s) => recoger_llamadas_sitio(s, v),
        CE::Binario(_, a, b) => {
            recoger_llamadas_expr(a, v);
            recoger_llamadas_expr(b, v);
        }
        CE::Unario(_, a) | CE::Cast(a) | CE::Entrada(a) => recoger_llamadas_expr(a, v),
        CE::Const(_) | CE::Bandera(_) => {}
    }
}
