//! Emision de ensamblador a partir de las instrucciones que la maquina declara.
//!
//! Cada operacion (cargar una constante, sumar, leer memoria, saltar si...) se resuelve buscando
//! en el [`Perfil`] una instruccion cuya semantica la haga. Cuando la maquina no la tiene, la
//! operacion se compone con otras (por ejemplo `mover` como `sumar 0`) y, si no hay forma, el
//! error explica que le falta a la maquina (RF-SGT-035).

use std::fmt::Write as _;

use lorenz_arch::modelo::{Instruccion, Maquina, RolRegistro, TipoOperando};
use lorenz_arch::semantica::ast::{OpBinario, OpUnario};

use crate::seleccion::{Cond, Dir, Forma, Fuente, Instr, Perfil, Reg, Salto};

/// Algo que la maquina no sabe hacer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Falta {
    pub operacion: String,
}

impl Falta {
    pub fn nueva(operacion: impl Into<String>) -> Self {
        Self {
            operacion: operacion.into(),
        }
    }
}

pub type R<T> = Result<T, Falta>;

/// Un valor conocido al compilar o por el ensamblador (una etiqueta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Valor {
    Num(i128),
    /// Expresion del ensamblador, normalmente una etiqueta.
    Sim(String),
}

impl Valor {
    fn texto(&self) -> String {
        match self {
            Valor::Num(n) => n.to_string(),
            Valor::Sim(s) => s.clone(),
        }
    }

    /// El valor desplazado a la derecha y recortado a `bits` bits.
    fn trozo(&self, desde: u32, bits: u32) -> Valor {
        let mascara = (1i128 << bits) - 1;
        match self {
            Valor::Num(n) => Valor::Num((n >> desde) & mascara),
            Valor::Sim(s) => Valor::Sim(format!("(({s}) >> {desde}) & {mascara}")),
        }
    }
}

/// Un lugar de memoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sitio {
    /// Direccion conocida al compilar.
    Fija(u64),
    /// Direccion en un registro.
    EnReg(String),
}

/// Una linea de ensamblador con la linea de codigo fuente de la que viene (RF-SGT-032).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linea {
    pub texto: String,
    pub fuente: Option<u32>,
}

pub struct Emisor<'a> {
    pub m: &'a Maquina,
    pub perfil: Perfil,
    pub lineas: Vec<Linea>,
    pub linea_fuente: Option<u32>,
    /// Registros que el compilador puede usar, en orden.
    pub pool: Vec<String>,
    /// Registro que apunta al inicio de los datos, si lo hay.
    pub gp: Option<(String, u64)>,
    /// Los saltos condicionales se generan siempre como salto largo.
    pub saltos_largos: bool,
    etiquetas: u32,
    pub palabra: u32,
}

fn operando<'i>(i: &'i Instruccion, n: &str) -> Option<&'i TipoOperando> {
    i.operandos.iter().find(|o| o.nombre == n).map(|o| &o.tipo)
}

fn nombre_op(r: &Reg) -> Option<&str> {
    match r {
        Reg::Operando(n) => Some(n),
        Reg::Fijo(_) => None,
    }
}

impl<'a> Emisor<'a> {
    pub fn nuevo(m: &'a Maquina) -> Self {
        let perfil = Perfil::de(m);
        Self {
            m,
            perfil,
            lineas: Vec::new(),
            linea_fuente: None,
            pool: Vec::new(),
            gp: None,
            saltos_largos: false,
            etiquetas: 0,
            palabra: m.bits_palabra,
        }
    }

    pub fn etiqueta_nueva(&mut self, prefijo: &str) -> String {
        self.etiquetas += 1;
        format!("_{prefijo}{}", self.etiquetas)
    }

    pub fn poner_etiqueta(&mut self, e: &str) {
        self.lineas.push(Linea {
            texto: format!("{e}:"),
            fuente: self.linea_fuente,
        });
    }

    pub fn directiva(&mut self, t: String) {
        self.lineas.push(Linea {
            texto: t,
            fuente: None,
        });
    }

    fn instruccion(&self, i: &Instr) -> &'a Instruccion {
        &self.m.instrucciones[i.indice]
    }

    fn emitir(&mut self, i: &Instr, valores: &[(&str, String)]) {
        let ins = self.instruccion(i);
        let mut t = ins.sintaxis.clone();
        for (k, v) in valores {
            t = t.replace(&format!("{{{k}}}"), v);
        }
        let mut s = String::new();
        let _ = write!(s, "    {t}");
        self.lineas.push(Linea {
            texto: s,
            fuente: self.linea_fuente,
        });
    }

    fn registros_permitidos(&self, i: &Instr, op: &str) -> Vec<String> {
        match operando(self.instruccion(i), op) {
            Some(TipoOperando::Registro { permitidos }) => permitidos.clone(),
            _ => Vec::new(),
        }
    }

    fn acepta(&self, i: &Instr, r: &Reg, valor: &str) -> bool {
        match nombre_op(r) {
            Some(op) => self.registros_permitidos(i, op).iter().any(|p| p == valor),
            None => false,
        }
    }

    fn rango_imm(&self, i: &Instr, op: &str) -> Option<(i128, i128)> {
        match operando(self.instruccion(i), op)? {
            TipoOperando::Inmediato { rango, .. } => {
                Some((i128::from(rango.0), i128::from(rango.1)))
            }
            TipoOperando::Direccion => Some((0, (1i128 << self.m.bits_direccion) - 1)),
            _ => None,
        }
    }

    // ------------------------------------------------------------------
    // Registros disponibles
    // ------------------------------------------------------------------

    /// Elige los registros que el compilador usara: los generales que aceptan todas las
    /// instrucciones nucleares (RF-SGT-025).
    pub fn preparar_registros(&mut self) -> R<()> {
        let generales: Vec<String> = self
            .m
            .registros
            .iter()
            .filter(|r| r.rol == RolRegistro::General && r.bits == self.m.bits_palabra)
            .map(|r| r.nombre.clone())
            .collect();
        let mut validos = generales.clone();
        for i in self.perfil.instrucciones.clone() {
            let nucleo = match &i.forma {
                Forma::Alu { op, .. } => matches!(
                    op,
                    OpBinario::Suma
                        | OpBinario::Resta
                        | OpBinario::Y
                        | OpBinario::O
                        | OpBinario::Xor
                        | OpBinario::Menor
                ),
                Forma::Mover { .. }
                | Forma::CargarImm { .. }
                | Forma::Leer { .. }
                | Forma::Escribir { .. }
                | Forma::SaltarReg(_) => true,
                Forma::SaltarSi {
                    cond: Cond::Comparar(..) | Cond::Cero(..),
                    ..
                } => true,
                _ => false,
            };
            if !nucleo {
                continue;
            }
            let ins = self.instruccion(&i);
            for o in &ins.operandos {
                if let TipoOperando::Registro { permitidos } = &o.tipo {
                    // Solo importa si la instruccion se puede usar con registros generales.
                    if permitidos.iter().any(|p| generales.contains(p)) {
                        validos.retain(|v| permitidos.contains(v));
                    }
                }
            }
        }
        if validos.len() < 4 {
            return Err(Falta::nueva(format!(
                "tener al menos 4 registros generales de {} bits que acepten a la vez las instrucciones de aritmetica, memoria y salto (hay {})",
                self.m.bits_palabra,
                validos.len()
            )));
        }
        self.pool = validos;
        Ok(())
    }

    /// Registro auxiliar que nunca guarda un valor entre sentencias.
    pub fn s0(&self) -> String {
        self.pool[0].clone()
    }

    pub fn s1(&self) -> String {
        self.pool[1].clone()
    }

    // ------------------------------------------------------------------
    // Mover y constantes
    // ------------------------------------------------------------------

    pub fn mover(&mut self, d: &str, s: &str) -> R<()> {
        if d == s {
            return Ok(());
        }
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| matches!(f, Forma::Mover { .. }))
            .cloned()
            .collect();
        for i in &candidatas {
            if let Forma::Mover { d: rd, s: rs } = &i.forma {
                if self.acepta(i, rd, d) && self.acepta(i, rs, s) {
                    let (od, os) = (
                        nombre_op(rd).unwrap().to_string(),
                        nombre_op(rs).unwrap().to_string(),
                    );
                    self.emitir(i, &[(&od, d.to_string()), (&os, s.to_string())]);
                    return Ok(());
                }
            }
        }
        // Sin MOV: sumar cero, o hacer `s | s`.
        if self.alu_imm(OpBinario::Suma, d, s, 0).is_ok() {
            return Ok(());
        }
        if self.alu_rr(OpBinario::O, d, s, s).is_ok() {
            return Ok(());
        }
        Err(Falta::nueva(
            "copiar un registro en otro (`rd = rs`, o `rd = rs + 0`)",
        ))
    }

    fn cargar_chico(&mut self, d: &str, v: i128) -> bool {
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| matches!(f, Forma::CargarImm { .. }))
            .cloned()
            .collect();
        for i in &candidatas {
            if let Forma::CargarImm { d: rd, v: ov } = &i.forma {
                if !self.acepta(i, rd, d) {
                    continue;
                }
                let Some((lo, hi)) = self.rango_imm(i, ov) else {
                    continue;
                };
                if matches!(
                    operando(self.instruccion(i), ov),
                    Some(TipoOperando::Direccion)
                ) {
                    // Una direccion cabe entera: sirve para cargar cualquier valor de palabra.
                }
                if v >= lo && v <= hi {
                    let od = nombre_op(rd).unwrap().to_string();
                    self.emitir(i, &[(&od, d.to_string()), (ov, v.to_string())]);
                    return true;
                }
            }
        }
        for i in self.perfil.instrucciones.clone() {
            if let Forma::CargarLiteral { d: rd, n } = &i.forma {
                if i128::from(*n) == v && self.acepta(&i, rd, d) {
                    let od = nombre_op(rd).unwrap().to_string();
                    self.emitir(&i, &[(&od, d.to_string())]);
                    return true;
                }
            }
        }
        false
    }

    /// Mayor constante sin signo que carga una sola instruccion.
    fn maximo_carga(&self, d: &str) -> i128 {
        let mut mejor = -1;
        for i in self
            .perfil
            .con_forma(|f| matches!(f, Forma::CargarImm { .. }))
        {
            if let Forma::CargarImm { d: rd, v } = &i.forma {
                if self.acepta(i, rd, d) {
                    if let Some((lo, hi)) = self.rango_imm(i, v) {
                        if lo <= 0 {
                            mejor = mejor.max(hi);
                        }
                    }
                }
            }
        }
        mejor
    }

    /// Carga `v` en `d`. Si no cabe en un inmediato, lo arma con trozos: carga el trozo alto y
    /// repite "duplicar hasta dejar sitio, sumar el trozo siguiente".
    pub fn cargar(&mut self, d: &str, v: &Valor) -> R<()> {
        if let Valor::Num(n) = v {
            let n = *n & ((1i128 << self.palabra) - 1);
            if self.cargar_chico(d, n) {
                return Ok(());
            }
            let con_signo = n - (1i128 << self.palabra);
            if self.cargar_chico(d, con_signo) {
                return Ok(());
            }
        }
        let tope = self.maximo_carga(d);
        if tope < 1 {
            return Err(Falta::nueva(
                "cargar una constante en un registro (`rd = imm`)",
            ));
        }
        // Trozos de `k` bits: lo que cabe en la carga y en una suma de inmediato.
        let k_carga = 127 - (tope as u128).leading_zeros();
        let k_carga = k_carga + 1;
        let k_suma = self.bits_suma_inmediata(d);
        let k = if k_suma > 0 {
            k_carga.min(k_suma)
        } else {
            k_carga.min(self.palabra)
        };
        let total = self.palabra;
        // El trozo mas alto puede ser mas ancho que k: lo que admita la carga.
        let n_trozos = if total <= k_carga {
            1
        } else {
            1 + (total - k_carga).div_ceil(k)
        };
        let alto_bits = total - (n_trozos - 1) * k;
        let alto = v.trozo(total - alto_bits, alto_bits);
        let Valor::Num(a) = alto else {
            return self.cargar_por_trozos_simbolicos(d, v, k, n_trozos, alto_bits);
        };
        if !self.cargar_chico(d, a) {
            return Err(Falta::nueva(
                "cargar una constante en un registro (`rd = imm`)",
            ));
        }
        for t in (0..n_trozos - 1).rev() {
            for _ in 0..k {
                self.duplicar(d)?;
            }
            let Valor::Num(c) = v.trozo(t * k, k) else {
                unreachable!()
            };
            if c != 0 {
                self.sumar_const(d, c)?;
            }
        }
        Ok(())
    }

    fn cargar_por_trozos_simbolicos(
        &mut self,
        d: &str,
        v: &Valor,
        k: u32,
        n: u32,
        alto_bits: u32,
    ) -> R<()> {
        let total = self.palabra;
        let alto = v.trozo(total - alto_bits, alto_bits);
        if !self.cargar_expr(d, &alto) {
            return Err(Falta::nueva(
                "cargar una constante en un registro (`rd = imm`)",
            ));
        }
        for t in (0..n - 1).rev() {
            for _ in 0..k {
                self.duplicar(d)?;
            }
            let trozo = v.trozo(t * k, k);
            let s = self.s_aux(d);
            if !self.cargar_expr(&s, &trozo) {
                return Err(Falta::nueva(
                    "cargar una constante en un registro (`rd = imm`)",
                ));
            }
            self.alu_rr(OpBinario::Suma, d, d, &s)?;
        }
        Ok(())
    }

    /// Carga una expresion del ensamblador con una instruccion de carga de inmediato.
    fn cargar_expr(&mut self, d: &str, v: &Valor) -> bool {
        if let Valor::Num(n) = v {
            return self.cargar_chico(d, *n);
        }
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| matches!(f, Forma::CargarImm { .. }))
            .cloned()
            .collect();
        for i in &candidatas {
            if let Forma::CargarImm { d: rd, v: ov } = &i.forma {
                if self.acepta(i, rd, d) && self.rango_imm(i, ov).is_some() {
                    let od = nombre_op(rd).unwrap().to_string();
                    self.emitir(i, &[(&od, d.to_string()), (ov, v.texto())]);
                    return true;
                }
            }
        }
        false
    }

    /// Un registro auxiliar distinto de `d`.
    fn s_aux(&self, d: &str) -> String {
        if self.pool[0] != d {
            self.pool[0].clone()
        } else {
            self.pool[1].clone()
        }
    }

    fn bits_suma_inmediata(&self, d: &str) -> u32 {
        let mut mejor = 0;
        for i in self.perfil.con_forma(|f| {
            matches!(
                f,
                Forma::Alu {
                    op: OpBinario::Suma,
                    b: Fuente::Imm(_),
                    ..
                }
            )
        }) {
            if let Forma::Alu {
                d: rd,
                b: Fuente::Imm(o),
                ..
            } = &i.forma
            {
                if self.acepta(i, rd, d) {
                    if let Some((_, hi)) = self.rango_imm(i, o) {
                        if hi > 0 {
                            mejor = mejor.max(
                                128 - (hi as u128).leading_zeros()
                                    - u32::from((hi + 1).count_ones() != 1),
                            );
                        }
                    }
                }
            }
        }
        mejor
    }

    pub fn duplicar(&mut self, d: &str) -> R<()> {
        // `d = d << 1` si hay desplazamiento; si no, `d + d`.
        if self.alu_imm(OpBinario::DesplazaIzq, d, d, 1).is_ok() {
            return Ok(());
        }
        self.alu_rr(OpBinario::Suma, d, d, d)
    }

    pub fn sumar_const(&mut self, d: &str, c: i128) -> R<()> {
        if self.alu_imm(OpBinario::Suma, d, d, c).is_ok() {
            return Ok(());
        }
        let s = self.s_aux(d);
        self.cargar(&s, &Valor::Num(c))?;
        self.alu_rr(OpBinario::Suma, d, d, &s)
    }

    // ------------------------------------------------------------------
    // Aritmetica
    // ------------------------------------------------------------------

    /// `d = a op b` con tres registros.
    pub fn alu_rr(&mut self, op: OpBinario, d: &str, a: &str, b: &str) -> R<()> {
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| matches!(f, Forma::Alu { op: o, b: Fuente::Reg(_), .. } if *o == op))
            .cloned()
            .collect();
        let conmuta = matches!(
            op,
            OpBinario::Suma | OpBinario::Y | OpBinario::O | OpBinario::Xor
        );
        for i in &candidatas {
            let Forma::Alu {
                d: rd,
                a: ra,
                b: Fuente::Reg(rb),
                ..
            } = &i.forma
            else {
                continue;
            };
            let (Some(od), Some(oa), Some(ob)) = (nombre_op(rd), nombre_op(ra), nombre_op(rb))
            else {
                continue;
            };
            if od != oa {
                // Tres direcciones.
                if self.acepta(i, rd, d) && self.acepta(i, ra, a) && self.acepta(i, rb, b) {
                    self.emitir(i, &[(od, d.into()), (oa, a.into()), (ob, b.into())]);
                    return Ok(());
                }
            } else if self.acepta(i, rd, d) && self.acepta(i, rb, b) {
                // Dos direcciones: `d = d op b`.
                if d == a {
                    self.emitir(i, &[(od, d.into()), (ob, b.into())]);
                    return Ok(());
                }
                if d != b {
                    self.mover(d, a)?;
                    self.emitir(i, &[(od, d.into()), (ob, b.into())]);
                    return Ok(());
                }
                if conmuta {
                    self.emitir(i, &[(od, d.into()), (ob, a.into())]);
                    return Ok(());
                }
                let s = self.libre_distinto(&[a, b, d]);
                self.mover(&s, a)?;
                self.emitir(i, &[(od, s.clone()), (ob, b.into())]);
                self.mover(d, &s)?;
                return Ok(());
            }
        }
        Err(Falta::nueva(format!(
            "calcular `rd = rs {} rt` entre registros",
            op.simbolo()
        )))
    }

    /// Si la maquina tiene alguna instruccion `rd = rs op rt` entre registros.
    pub fn tiene_alu_rr(&self, op: OpBinario) -> bool {
        self.perfil
            .con_forma(|f| matches!(f, Forma::Alu { op: o, b: Fuente::Reg(_), .. } if *o == op))
            .next()
            .is_some()
    }

    fn libre_distinto(&self, ocupados: &[&str]) -> String {
        self.pool
            .iter()
            .find(|r| !ocupados.contains(&r.as_str()))
            .cloned()
            .unwrap_or_else(|| self.pool[0].clone())
    }

    /// `d = a op c` con una constante pequena en la instruccion.
    pub fn alu_imm(&mut self, op: OpBinario, d: &str, a: &str, c: i128) -> R<()> {
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| matches!(f, Forma::Alu { op: o, b: Fuente::Imm(_), .. } if *o == op))
            .cloned()
            .collect();
        for i in &candidatas {
            let Forma::Alu {
                d: rd,
                a: ra,
                b: Fuente::Imm(oi),
                ..
            } = &i.forma
            else {
                continue;
            };
            let (Some(od), Some(oa)) = (nombre_op(rd), nombre_op(ra)) else {
                continue;
            };
            let Some((lo, hi)) = self.rango_imm(i, oi) else {
                continue;
            };
            if c < lo || c > hi {
                continue;
            }
            if od != oa {
                if self.acepta(i, rd, d) && self.acepta(i, ra, a) {
                    self.emitir(i, &[(od, d.into()), (oa, a.into()), (oi, c.to_string())]);
                    return Ok(());
                }
            } else if self.acepta(i, rd, d) {
                self.mover(d, a)?;
                self.emitir(i, &[(od, d.into()), (oi, c.to_string())]);
                return Ok(());
            }
        }
        Err(Falta::nueva(format!(
            "calcular `rd = rs {} constante` con la constante dentro de la instruccion",
            op.simbolo()
        )))
    }

    /// `d = a op b` donde `b` es una constante cualquiera.
    pub fn alu_const(&mut self, op: OpBinario, d: &str, a: &str, c: i128) -> R<()> {
        if self.alu_imm(op, d, a, c).is_ok() {
            return Ok(());
        }
        let s = self.libre_distinto(&[a, d]);
        self.cargar(&s, &Valor::Num(c))?;
        self.alu_rr(op, d, a, &s)
    }

    /// Cambio de signo o complemento.
    pub fn unaria(&mut self, op: OpUnario, d: &str, a: &str) -> R<()> {
        for i in self.perfil.instrucciones.clone() {
            if let Forma::Unaria {
                op: o,
                d: rd,
                a: ra,
            } = &i.forma
            {
                if *o != op {
                    continue;
                }
                let (Some(od), Some(oa)) = (nombre_op(rd), nombre_op(ra)) else {
                    continue;
                };
                if od != oa && self.acepta(&i, rd, d) && self.acepta(&i, ra, a) {
                    self.emitir(&i, &[(od, d.into()), (oa, a.into())]);
                    return Ok(());
                }
            }
        }
        match op {
            OpUnario::Menos => {
                let s = self.libre_distinto(&[a, d]);
                self.cargar(&s, &Valor::Num(0))?;
                self.alu_rr(OpBinario::Resta, d, &s, a)
            }
            OpUnario::NegacionBits => {
                let ones = (1i128 << self.palabra) - 1;
                self.alu_const(OpBinario::Xor, d, a, ones)
            }
            OpUnario::NegacionLogica => Err(Falta::nueva("negar un valor logico")),
        }
    }

    // ------------------------------------------------------------------
    // Memoria
    // ------------------------------------------------------------------

    fn forma_leer(&self) -> Vec<Instr> {
        self.perfil
            .con_forma(|f| matches!(f, Forma::Leer { .. }))
            .cloned()
            .collect()
    }

    fn forma_escribir(&self) -> Vec<Instr> {
        self.perfil
            .con_forma(|f| matches!(f, Forma::Escribir { .. }))
            .cloned()
            .collect()
    }

    /// Resuelve un sitio en una forma de acceso: devuelve los valores de operandos y, si hace
    /// falta, el registro de direccion que hay que preparar antes.
    fn acceso(
        &mut self,
        sitio: &Sitio,
        forma: &Dir,
        i: &Instr,
        reg_dato: &str,
    ) -> R<Option<Vec<(String, String)>>> {
        match (sitio, forma) {
            (Sitio::Fija(a), Dir::Absoluta(op)) => {
                let (lo, hi) = self.rango_imm(i, op).unwrap_or((0, 0));
                if i128::from(*a) >= lo && i128::from(*a) <= hi {
                    return Ok(Some(vec![(op.clone(), a.to_string())]));
                }
                Ok(None)
            }
            (Sitio::Fija(a), Dir::Indirecta { base, desp }) => {
                let Some(ob) = nombre_op(base) else {
                    return Ok(None);
                };
                // Con registro base de datos y desplazamiento pequeno.
                if let (Some((gp, origen)), Some(od)) = (self.gp.clone(), desp) {
                    let (lo, hi) = self.rango_imm(i, od).unwrap_or((0, -1));
                    let rel = i128::from(*a) - i128::from(origen);
                    if rel >= lo && rel <= hi && self.acepta(i, base, &gp) {
                        return Ok(Some(vec![
                            (ob.to_string(), gp),
                            (od.clone(), rel.to_string()),
                        ]));
                    }
                }
                let s = if reg_dato == self.pool[1] {
                    self.s0()
                } else {
                    self.s1()
                };
                if !self.acepta(i, base, &s) {
                    return Ok(None);
                }
                self.cargar(&s, &Valor::Num(i128::from(*a)))?;
                let mut v = vec![(ob.to_string(), s)];
                if let Some(od) = desp {
                    v.push((od.clone(), "0".to_string()));
                }
                Ok(Some(v))
            }
            (Sitio::EnReg(r), Dir::Indirecta { base, desp }) => {
                let Some(ob) = nombre_op(base) else {
                    return Ok(None);
                };
                if !self.acepta(i, base, r) {
                    return Ok(None);
                }
                let mut v = vec![(ob.to_string(), r.clone())];
                if let Some(od) = desp {
                    v.push((od.clone(), "0".to_string()));
                }
                Ok(Some(v))
            }
            (Sitio::EnReg(_), Dir::Absoluta(_)) => Ok(None),
        }
    }

    pub fn leer(&mut self, d: &str, sitio: &Sitio) -> R<()> {
        for i in self.forma_leer() {
            let Forma::Leer { d: rd, dir } = &i.forma else {
                continue;
            };
            let Some(od) = nombre_op(rd) else { continue };
            if !self.acepta(&i, rd, d) {
                continue;
            }
            if let Some(mut v) = self.acceso(sitio, dir, &i, d)? {
                v.push((od.to_string(), d.to_string()));
                let v: Vec<(&str, String)> =
                    v.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
                self.emitir(&i, &v);
                return Ok(());
            }
        }
        Err(Falta::nueva(match sitio {
            Sitio::Fija(_) => "leer una palabra de memoria (`rd = mem[...]`)".to_string(),
            Sitio::EnReg(_) => "leer memoria por un registro de direccion (`rd = mem[rs + off]`), necesario para punteros y elementos de arreglo".to_string(),
        }))
    }

    pub fn escribir(&mut self, sitio: &Sitio, s: &str) -> R<()> {
        for i in self.forma_escribir() {
            let Forma::Escribir { s: rs, dir } = &i.forma else {
                continue;
            };
            let Some(os) = nombre_op(rs) else { continue };
            if !self.acepta(&i, rs, s) {
                continue;
            }
            if let Some(mut v) = self.acceso(sitio, dir, &i, s)? {
                v.push((os.to_string(), s.to_string()));
                let v: Vec<(&str, String)> =
                    v.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
                self.emitir(&i, &v);
                return Ok(());
            }
        }
        Err(Falta::nueva(match sitio {
            Sitio::Fija(_) => "escribir una palabra en memoria (`mem[...] = rs`)".to_string(),
            Sitio::EnReg(_) => "escribir memoria por un registro de direccion (`mem[rs + off] = rd`), necesario para punteros y elementos de arreglo".to_string(),
        }))
    }

    // ------------------------------------------------------------------
    // Saltos
    // ------------------------------------------------------------------

    pub fn saltar(&mut self, etiqueta: &str) -> R<()> {
        for i in self.perfil.instrucciones.clone() {
            if let Forma::Saltar(s) = &i.forma {
                let (Salto::Absoluto(o) | Salto::Relativo(o)) = s;
                self.emitir(&i, &[(o, etiqueta.to_string())]);
                return Ok(());
            }
        }
        Err(Falta::nueva("saltar a una direccion (`pc = dir`)"))
    }

    pub fn saltar_reg(&mut self, r: &str) -> R<()> {
        for i in self.perfil.instrucciones.clone() {
            if let Forma::SaltarReg(rr) = &i.forma {
                if self.acepta(&i, rr, r) {
                    let o = nombre_op(rr).unwrap().to_string();
                    self.emitir(&i, &[(&o, r.to_string())]);
                    return Ok(());
                }
            }
        }
        Err(Falta::nueva("saltar a la direccion que hay en un registro (`pc = rs`), necesario para volver de una funcion"))
    }

    pub fn detener(&mut self) -> R<()> {
        match self.perfil.detener().cloned() {
            Some(i) => {
                self.emitir(&i, &[]);
                Ok(())
            }
            None => Err(Falta::nueva("detener la maquina (`detener`)")),
        }
    }

    /// Salta a `etiqueta` si `a op b` (comparacion entre registros, sin signo) es `cuando`.
    pub fn saltar_si(
        &mut self,
        op: OpBinario,
        a: &str,
        b: &str,
        cuando: bool,
        etiqueta: &str,
    ) -> R<()> {
        let op = if cuando { op } else { negar(op) };
        // 1. Una instruccion que compare esos dos registros con ese operador, o con los
        //    operandos intercambiados.
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| {
                matches!(
                    f,
                    Forma::SaltarSi {
                        cond: Cond::Comparar(..),
                        ..
                    }
                )
            })
            .cloned()
            .collect();
        for (o, x, y) in [(op, a, b), (intercambiar(op), b, a)] {
            for i in &candidatas {
                let Forma::SaltarSi {
                    cond: Cond::Comparar(co, ra, rb),
                    salto,
                } = &i.forma
                else {
                    continue;
                };
                if *co != o {
                    continue;
                }
                let (Some(oa), Some(ob)) = (nombre_op(ra), nombre_op(rb)) else {
                    continue;
                };
                if !self.acepta(i, ra, x) || !self.acepta(i, rb, y) {
                    continue;
                }
                return self.salto_condicional(
                    i,
                    salto,
                    &[(oa, x.to_string()), (ob, y.to_string())],
                    etiqueta,
                );
            }
        }
        // 2. Ordenar con un booleano (`rd = a < b`) y comparar con cero.
        if matches!(
            op,
            OpBinario::Menor | OpBinario::MenorIgual | OpBinario::Mayor | OpBinario::MayorIgual
        ) {
            let (menor, x, y, invertir) = match op {
                OpBinario::Menor => (true, a, b, false),
                OpBinario::Mayor => (true, b, a, false),
                OpBinario::MayorIgual => (true, a, b, true),
                _ => (true, b, a, true),
            };
            let _ = menor;
            let t = self.s0();
            if t != x && t != y {
                self.alu_rr(OpBinario::Menor, &t, x, y)?;
                return self.saltar_si_cero(&t, invertir, etiqueta);
            }
        }
        Err(Falta::nueva(format!(
            "saltar segun la comparacion `{}` entre dos registros (una instruccion tipo `si rs {} rt: pc = ...`)",
            op.simbolo(),
            op.simbolo()
        )))
    }

    /// Salta si el registro vale cero (`cero = true`) o no vale cero.
    pub fn saltar_si_cero(&mut self, r: &str, cero: bool, etiqueta: &str) -> R<()> {
        let candidatas: Vec<Instr> = self
            .perfil
            .con_forma(|f| {
                matches!(
                    f,
                    Forma::SaltarSi {
                        cond: Cond::Cero(..),
                        ..
                    }
                )
            })
            .cloned()
            .collect();
        for i in &candidatas {
            let Forma::SaltarSi {
                cond: Cond::Cero(rr, es_cero),
                salto,
            } = &i.forma
            else {
                continue;
            };
            if *es_cero == cero && self.acepta(i, rr, r) {
                let o = nombre_op(rr).unwrap().to_string();
                return self.salto_condicional(i, salto, &[(&o, r.to_string())], etiqueta);
            }
        }
        // Comparar con un registro que vale cero.
        let z = if self.pool[1] != r {
            self.s1()
        } else {
            self.s0()
        };
        self.cargar(&z, &Valor::Num(0))?;
        self.saltar_si(
            if cero {
                OpBinario::Igual
            } else {
                OpBinario::Distinto
            },
            r,
            &z,
            true,
            etiqueta,
        )
    }

    fn salto_condicional(
        &mut self,
        i: &Instr,
        salto: &Salto,
        ops: &[(&str, String)],
        etiqueta: &str,
    ) -> R<()> {
        let (Salto::Absoluto(os) | Salto::Relativo(os)) = salto;
        let mut v: Vec<(&str, String)> = ops.to_vec();
        if self.saltos_largos {
            // Si NO se cumple, salta por encima de un salto largo.
            let Some(inv) = self.instruccion_inversa(i) else {
                return Err(Falta::nueva(
                    "invertir una condicion de salto para generar un salto largo",
                ));
            };
            let paso = self.etiqueta_nueva("p");
            let Forma::SaltarSi { salto: s2, cond } = &inv.forma else {
                unreachable!()
            };
            let (Salto::Absoluto(o2) | Salto::Relativo(o2)) = s2;
            let mut ops2: Vec<(String, String)> = Vec::new();
            match (cond, &i.forma) {
                (
                    Cond::Comparar(_, ra, rb),
                    Forma::SaltarSi {
                        cond: Cond::Comparar(_, ra0, rb0),
                        ..
                    },
                ) => {
                    let (a0, b0) = (nombre_op(ra0).unwrap(), nombre_op(rb0).unwrap());
                    let val = |n: &str| ops.iter().find(|(k, _)| *k == n).unwrap().1.clone();
                    ops2.push((nombre_op(ra).unwrap().to_string(), val(a0)));
                    ops2.push((nombre_op(rb).unwrap().to_string(), val(b0)));
                }
                (
                    Cond::Cero(rr, _),
                    Forma::SaltarSi {
                        cond: Cond::Cero(r0, _),
                        ..
                    },
                ) => {
                    let val = |n: &str| ops.iter().find(|(k, _)| *k == n).unwrap().1.clone();
                    ops2.push((
                        nombre_op(rr).unwrap().to_string(),
                        val(nombre_op(r0).unwrap()),
                    ));
                }
                (Cond::Bandera(..), _) => {}
                _ => {
                    return Err(Falta::nueva(
                        "invertir una condicion de salto para generar un salto largo",
                    ))
                }
            }
            let mut v2: Vec<(&str, String)> =
                ops2.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
            v2.push((o2.as_str(), paso.clone()));
            self.emitir(&inv, &v2);
            self.saltar(etiqueta)?;
            self.poner_etiqueta(&paso);
            return Ok(());
        }
        v.push((os.as_str(), etiqueta.to_string()));
        self.emitir(i, &v);
        Ok(())
    }

    /// Otra instruccion que salta justo cuando `i` no salta.
    fn instruccion_inversa(&self, i: &Instr) -> Option<Instr> {
        let Forma::SaltarSi { cond, .. } = &i.forma else {
            return None;
        };
        for j in &self.perfil.instrucciones {
            let Forma::SaltarSi { cond: c2, .. } = &j.forma else {
                continue;
            };
            let ok = match (cond, c2) {
                (Cond::Comparar(a, ..), Cond::Comparar(b, ..)) => *b == negar(*a),
                (Cond::Cero(_, x), Cond::Cero(_, y)) => x != y,
                (Cond::Bandera(n, x), Cond::Bandera(m, y)) => n == m && x != y,
                _ => false,
            };
            if ok {
                return Some(j.clone());
            }
        }
        None
    }
}

/// Si el perfil tiene alguna instruccion `rd = rs op rt` entre registros.
pub fn perfil_tiene_alu_rr(p: &Perfil, op: OpBinario) -> bool {
    p.con_forma(|f| matches!(f, Forma::Alu { op: o, b: Fuente::Reg(_), .. } if *o == op))
        .next()
        .is_some()
}

pub fn negar(op: OpBinario) -> OpBinario {
    match op {
        OpBinario::Igual => OpBinario::Distinto,
        OpBinario::Distinto => OpBinario::Igual,
        OpBinario::Menor => OpBinario::MayorIgual,
        OpBinario::MayorIgual => OpBinario::Menor,
        OpBinario::Mayor => OpBinario::MenorIgual,
        OpBinario::MenorIgual => OpBinario::Mayor,
        o => o,
    }
}

pub fn intercambiar(op: OpBinario) -> OpBinario {
    match op {
        OpBinario::Menor => OpBinario::Mayor,
        OpBinario::Mayor => OpBinario::Menor,
        OpBinario::MenorIgual => OpBinario::MayorIgual,
        OpBinario::MayorIgual => OpBinario::MenorIgual,
        o => o,
    }
}
