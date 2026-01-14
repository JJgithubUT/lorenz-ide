//! Reconocimiento de instrucciones por su semantica declarada (RF-SGT-024).
//!
//! El compilador no conoce ninguna arquitectura: lee la `semantica` de cada instruccion y la
//! clasifica en una forma (mover, sumar, cargar, saltar si...). Despues, para cada operacion que
//! necesita, pide al perfil una instruccion de esa forma. Si no hay ninguna, el error nombra la
//! operacion y la maquina (RF-SGT-035), no la instruccion que le habria hecho falta.

use lorenz_arch::modelo::{Instruccion, Maquina, RolRegistro, TipoOperando};
use lorenz_arch::semantica::ast::{Destino, Expr, OpBinario, OpUnario, Sentencia};

/// Un registro: o bien un operando de la instruccion, o bien uno concreto de la maquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reg {
    Operando(String),
    Fijo(String),
}

/// Segundo origen de una operacion: otro registro o una constante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fuente {
    Reg(Reg),
    /// Nombre de un operando inmediato.
    Imm(String),
    Literal(u64),
}

/// Como se forma una direccion de memoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dir {
    /// Direccion absoluta que viene en el operando de ese nombre.
    Absoluta(String),
    /// Registro base mas un desplazamiento opcional (nombre de operando).
    Indirecta { base: Reg, desp: Option<String> },
}

/// A donde salta una instruccion de salto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Salto {
    Absoluto(String),
    Relativo(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cond {
    /// `a OP b` entre dos registros.
    Comparar(OpBinario, Reg, Reg),
    /// Una bandera encendida (`true`) o apagada (`false`).
    Bandera(String, bool),
    /// Un registro contra cero.
    Cero(Reg, bool),
}

/// Puerto de una instruccion de entrada o salida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Puerto {
    /// Lo elige el operando de ese nombre.
    Operando(String),
    /// Fijo en la semantica de la instruccion.
    Fijo(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forma {
    Mover {
        d: Reg,
        s: Reg,
    },
    /// `d = operando` (inmediato o direccion).
    CargarImm {
        d: Reg,
        v: String,
    },
    /// `d = literal` fijo en la instruccion (por ejemplo `CLR`).
    CargarLiteral {
        d: Reg,
        n: u64,
    },
    /// `d = a op b`. Si `a == d` la instruccion es de dos operandos.
    Alu {
        op: OpBinario,
        d: Reg,
        a: Reg,
        b: Fuente,
    },
    Unaria {
        op: OpUnario,
        d: Reg,
        a: Reg,
    },
    Leer {
        d: Reg,
        dir: Dir,
    },
    Escribir {
        s: Reg,
        dir: Dir,
    },
    Saltar(Salto),
    SaltarReg(Reg),
    SaltarSi {
        cond: Cond,
        salto: Salto,
    },
    Entrada {
        d: Reg,
        puerto: Puerto,
    },
    Salida {
        s: Reg,
        puerto: Puerto,
    },
    Detener,
    /// `ie = 1` (habilitar) o `ie = 0` (inhabilitar) las interrupciones.
    Interrupciones(bool),
    /// Vuelve de una interrupcion: asigna el contador de programa y habilita las interrupciones.
    RetornoInterrupcion,
    Nada,
    /// Cualquier otra cosa: el compilador no la usa.
    Otra,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instr {
    /// Posicion en `Maquina::instrucciones`.
    pub indice: usize,
    pub mnemonico: String,
    pub forma: Forma,
}

#[derive(Debug, Clone)]
pub struct Perfil {
    pub instrucciones: Vec<Instr>,
}

enum Nombre {
    Operando(TipoOperando),
    Registro(String, RolRegistro),
    Bandera,
    Pc,
    Sp,
    Otro,
}

struct Ctx<'a> {
    m: &'a Maquina,
    i: &'a Instruccion,
}

impl Ctx<'_> {
    fn clase(&self, n: &str) -> Nombre {
        if let Some(o) = self.i.operandos.iter().find(|o| o.nombre == n) {
            return Nombre::Operando(o.tipo.clone());
        }
        if n == "pc" {
            return Nombre::Pc;
        }
        if n == "sp" {
            return Nombre::Sp;
        }
        if let Some(r) = self.m.registro(n) {
            return match r.rol {
                RolRegistro::ContadorPrograma => Nombre::Pc,
                RolRegistro::PunteroPila => Nombre::Sp,
                rol => Nombre::Registro(r.nombre.clone(), rol),
            };
        }
        if self.m.banderas.iter().any(|b| b.nombre == n) {
            return Nombre::Bandera;
        }
        Nombre::Otro
    }

    /// Un registro usable como dato: operando de tipo registro o registro general fijo.
    fn reg(&self, e: &Expr) -> Option<Reg> {
        let Expr::Nombre(n) = e else { return None };
        match self.clase(n) {
            Nombre::Operando(TipoOperando::Registro { .. }) => Some(Reg::Operando(n.clone())),
            Nombre::Registro(r, RolRegistro::General | RolRegistro::Indirecto) => {
                Some(Reg::Fijo(r))
            }
            _ => None,
        }
    }

    /// Un registro especial (puntero de pila, banderas...) por nombre o alias.
    fn especial(&self, n: &str) -> Option<String> {
        match self.clase(n) {
            Nombre::Sp => self
                .m
                .registro_con_rol(RolRegistro::PunteroPila)
                .map(|r| r.nombre.clone()),
            Nombre::Registro(r, rol)
                if !matches!(rol, RolRegistro::General | RolRegistro::Indirecto) =>
            {
                Some(r)
            }
            _ => None,
        }
    }

    fn es_pc(&self, e: &Expr) -> bool {
        matches!(e, Expr::Nombre(n) if matches!(self.clase(n), Nombre::Pc))
    }

    fn operando_inmediato(&self, e: &Expr) -> Option<String> {
        let Expr::Nombre(n) = e else { return None };
        match self.clase(n) {
            Nombre::Operando(TipoOperando::Inmediato { .. } | TipoOperando::Direccion) => {
                Some(n.clone())
            }
            _ => None,
        }
    }

    fn fuente(&self, e: &Expr) -> Option<Fuente> {
        if let Some(r) = self.reg(e) {
            return Some(Fuente::Reg(r));
        }
        if let Some(i) = self.operando_inmediato(e) {
            return Some(Fuente::Imm(i));
        }
        if let Expr::Literal(n) = e {
            return Some(Fuente::Literal(*n));
        }
        None
    }

    fn dir(&self, e: &Expr) -> Option<Dir> {
        if let Some(n) = self.operando_inmediato(e) {
            return Some(Dir::Absoluta(n));
        }
        if let Some(base) = self.reg(e) {
            return Some(Dir::Indirecta { base, desp: None });
        }
        if let Expr::Binario(OpBinario::Suma, a, b) = e {
            if let (Some(base), Some(off)) = (self.reg(a), self.operando_inmediato(b)) {
                return Some(Dir::Indirecta {
                    base,
                    desp: Some(off),
                });
            }
        }
        None
    }

    fn salto(&self, e: &Expr) -> Option<Salto> {
        if let Expr::Nombre(n) = e {
            if let Nombre::Operando(TipoOperando::Direccion | TipoOperando::Inmediato { .. }) =
                self.clase(n)
            {
                return Some(Salto::Absoluto(n.clone()));
            }
        }
        if let Expr::Nombre(n) = e {
            if let Nombre::Operando(TipoOperando::Desplazamiento) = self.clase(n) {
                return Some(Salto::Relativo(n.clone()));
            }
        }
        if let Expr::Binario(OpBinario::Suma, a, b) = e {
            if self.es_pc(a) {
                if let Expr::Nombre(n) = &**b {
                    if let Nombre::Operando(
                        TipoOperando::Desplazamiento | TipoOperando::Inmediato { .. },
                    ) = self.clase(n)
                    {
                        return Some(Salto::Relativo(n.clone()));
                    }
                }
            }
        }
        None
    }

    fn condicion(&self, e: &Expr) -> Option<Cond> {
        match e {
            Expr::Nombre(n) if matches!(self.clase(n), Nombre::Bandera) => {
                Some(Cond::Bandera(n.clone(), true))
            }
            Expr::Unario(OpUnario::NegacionLogica, x) => match &**x {
                Expr::Nombre(n) if matches!(self.clase(n), Nombre::Bandera) => {
                    Some(Cond::Bandera(n.clone(), false))
                }
                r => self.reg(r).map(|r| Cond::Cero(r, true)),
            },
            Expr::Binario(op, a, b)
                if matches!(
                    op,
                    OpBinario::Igual
                        | OpBinario::Distinto
                        | OpBinario::Menor
                        | OpBinario::MenorIgual
                        | OpBinario::Mayor
                        | OpBinario::MayorIgual
                ) =>
            {
                if let (Some(x), Expr::Literal(0)) = (self.reg(a), &**b) {
                    match op {
                        OpBinario::Igual => return Some(Cond::Cero(x, true)),
                        OpBinario::Distinto => return Some(Cond::Cero(x, false)),
                        _ => {}
                    }
                }
                Some(Cond::Comparar(*op, self.reg(a)?, self.reg(b)?))
            }
            e => self.reg(e).map(|r| Cond::Cero(r, false)),
        }
    }

    fn puerto(&self, e: &Expr) -> Option<Puerto> {
        match e {
            Expr::Literal(n) => Some(Puerto::Fijo(*n)),
            Expr::Nombre(n) if matches!(self.clase(n), Nombre::Operando(TipoOperando::Puerto)) => {
                Some(Puerto::Operando(n.clone()))
            }
            _ => None,
        }
    }

    fn asigna(&self, s: &[Sentencia], nombre: &str) -> bool {
        s.iter().any(|x| match x {
            Sentencia::Asignar(Destino::Nombre(n), _) => {
                n == nombre || (nombre == "pc" && matches!(self.clase(n), Nombre::Pc))
            }
            Sentencia::Si(_, c) => self.asigna(c, nombre),
            _ => false,
        })
    }

    fn forma(&self) -> Forma {
        let s = &self.i.programa.sentencias;
        if self.asigna(s, "ie") {
            return match s.as_slice() {
                [Sentencia::Asignar(_, Expr::Literal(v))] => Forma::Interrupciones(*v != 0),
                _ if self.asigna(s, "pc") => Forma::RetornoInterrupcion,
                _ => Forma::Otra,
            };
        }
        match s.as_slice() {
            [] => Forma::Nada,
            [Sentencia::Detener] => Forma::Detener,
            [Sentencia::Asignar(d, e)] => self.asignacion(d, e),
            [Sentencia::Si(c, cuerpo)] => match cuerpo.as_slice() {
                [Sentencia::Asignar(Destino::Nombre(p), e)]
                    if matches!(self.clase(p), Nombre::Pc) =>
                {
                    match (self.condicion(c), self.salto(e)) {
                        (Some(cond), Some(salto)) => Forma::SaltarSi { cond, salto },
                        _ => Forma::Otra,
                    }
                }
                _ => Forma::Otra,
            },
            _ => Forma::Otra,
        }
    }

    fn asignacion(&self, d: &Destino, e: &Expr) -> Forma {
        match d {
            Destino::Nombre(n) => match self.clase(n) {
                Nombre::Pc => {
                    if let Some(s) = self.salto(e) {
                        return Forma::Saltar(s);
                    }
                    if let Some(r) = self.reg(e) {
                        return Forma::SaltarReg(r);
                    }
                    Forma::Otra
                }
                Nombre::Operando(TipoOperando::Registro { .. })
                | Nombre::Registro(_, RolRegistro::General | RolRegistro::Indirecto) => {
                    let dst = self.reg(&Expr::Nombre(n.clone())).unwrap();
                    self.valor(dst, e)
                }
                Nombre::Sp | Nombre::Registro(..) => match (self.especial(n), self.reg(e)) {
                    (Some(d), Some(s)) => Forma::Mover { d: Reg::Fijo(d), s },
                    _ => Forma::Otra,
                },
                _ => Forma::Otra,
            },
            Destino::Mem(a) => match (self.dir(a), self.reg(e)) {
                (Some(dir), Some(s)) => Forma::Escribir { s, dir },
                _ => Forma::Otra,
            },
            Destino::Salida(p) => match (self.puerto(p), self.reg(e)) {
                (Some(puerto), Some(s)) => Forma::Salida { s, puerto },
                _ => Forma::Otra,
            },
        }
    }

    fn valor(&self, d: Reg, e: &Expr) -> Forma {
        match e {
            Expr::Literal(n) => Forma::CargarLiteral { d, n: *n },
            Expr::Nombre(n) => {
                if let Some(s) = self.reg(e) {
                    Forma::Mover { d, s }
                } else if let Some(s) = self.especial(n) {
                    Forma::Mover { d, s: Reg::Fijo(s) }
                } else if let Some(v) = self.operando_inmediato(e) {
                    Forma::CargarImm { d, v }
                } else {
                    Forma::Otra
                }
            }
            Expr::Mem(a) => match self.dir(a) {
                Some(dir) => Forma::Leer { d, dir },
                None => Forma::Otra,
            },
            Expr::Entrada(p) => match self.puerto(p) {
                Some(puerto) => Forma::Entrada { d, puerto },
                None => Forma::Otra,
            },
            Expr::Unario(op, a) => match self.reg(a) {
                Some(a) => Forma::Unaria { op: *op, d, a },
                None => Forma::Otra,
            },
            Expr::Binario(op, a, b) => match (self.reg(a), self.fuente(b)) {
                (Some(a), Some(b)) => Forma::Alu { op: *op, d, a, b },
                _ => Forma::Otra,
            },
        }
    }
}

impl Perfil {
    pub fn de(m: &Maquina) -> Self {
        let instrucciones = m
            .instrucciones
            .iter()
            .enumerate()
            .map(|(indice, i)| Instr {
                indice,
                mnemonico: i.mnemonico.clone(),
                forma: Ctx { m, i }.forma(),
            })
            .collect();
        Self { instrucciones }
    }

    pub fn con_forma(&self, f: impl Fn(&Forma) -> bool) -> impl Iterator<Item = &Instr> {
        self.instrucciones.iter().filter(move |i| f(&i.forma))
    }

    pub fn detener(&self) -> Option<&Instr> {
        self.con_forma(|f| matches!(f, Forma::Detener)).next()
    }
}
