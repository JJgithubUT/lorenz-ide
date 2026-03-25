//! Evaluador de referencia de la semantica (RF-TST-017).
//!
//! Esta escrito aparte del interprete del emulador, solo a partir de lo que dice
//! `docs/SEMANTICA.md`, para que la suite de conformidad compare dos lecturas independientes de
//! la misma especificacion. No comparte codigo con `lorenz-emu`.

use std::collections::{BTreeMap, BTreeSet};

use lorenz_arch::modelo::{Ciclos, Instruccion, Maquina, RolRegistro, TipoOperando};
use lorenz_arch::semantica::ast::{Destino, Expr, OpBinario, OpUnario, Sentencia};

/// Un operando ya decodificado, tal como lo ve la semantica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValorDeOperando {
    pub nombre: String,
    /// Indice dentro de los permitidos para un registro; el numero para el resto.
    pub valor: i128,
}

/// Todo lo que la semantica puede tocar.
#[derive(Debug, Clone)]
pub struct EstadoRef {
    /// Registros sin el de banderas.
    pub registros: BTreeMap<String, u64>,
    pub banderas: BTreeMap<String, bool>,
    pub ie: bool,
    /// Celdas escritas durante la instruccion.
    pub escrituras: BTreeMap<u64, u64>,
    /// Direcciones leidas antes de que la propia instruccion las escribiera.
    pub lecturas_iniciales: BTreeSet<u64>,
    /// Toda direccion que la instruccion toco, para saber si cae fuera de la RAM.
    pub accedidas: BTreeSet<u64>,
    pub salto: bool,
    pub detenida: bool,
}

/// Lo que dejo la instruccion.
#[derive(Debug, Clone)]
pub struct ResultadoRef {
    pub estado: EstadoRef,
    pub ciclos: u64,
}

/// Como la referencia ve la memoria: cuanto vale una celda y si se puede escribir en ella.
///
/// Cualquier funcion `Fn(u64) -> u64` sirve como memoria sin permisos; el ejecutor de referencia
/// da una que respeta las regiones.
pub trait AccesoMemoria {
    fn leer(&self, direccion: u64) -> Result<u64, ErrorRef>;
    fn comprobar_escritura(&self, direccion: u64) -> Result<(), ErrorRef>;
}

impl<F: Fn(u64) -> u64> AccesoMemoria for F {
    fn leer(&self, direccion: u64) -> Result<u64, ErrorRef> {
        Ok(self(direccion))
    }

    fn comprobar_escritura(&self, _direccion: u64) -> Result<(), ErrorRef> {
        Ok(())
    }
}

/// Por que la referencia no pudo terminar la instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorRef {
    /// Un acceso a memoria rechazado por la memoria que se dio.
    Memoria(lorenz_emu::ErrorEjecucion),
    DivisionEntreCero,
    NombreDesconocido(String),
    DestinoNoAsignable(String),
    /// Usa puertos: la referencia no los modela.
    UsaPuertos,
}

/// Si la semantica de la instruccion toca algun puerto.
pub fn usa_puertos(instruccion: &Instruccion) -> bool {
    fn en_expr(e: &Expr) -> bool {
        match e {
            Expr::Entrada(_) => true,
            Expr::Literal(_) | Expr::Nombre(_) => false,
            Expr::Mem(a) | Expr::Unario(_, a) => en_expr(a),
            Expr::Binario(_, a, b) => en_expr(a) || en_expr(b),
        }
    }
    fn en_sentencias(s: &[Sentencia]) -> bool {
        s.iter().any(|s| match s {
            Sentencia::Detener => false,
            Sentencia::Si(c, cuerpo) => en_expr(c) || en_sentencias(cuerpo),
            Sentencia::Asignar(d, e) => {
                en_expr(e)
                    || match d {
                        Destino::Nombre(_) => false,
                        Destino::Mem(a) => en_expr(a),
                        Destino::Salida(_) => true,
                    }
            }
        })
    }
    en_sentencias(&instruccion.programa.sentencias)
}

/// Banderas que la semantica asigna por su nombre.
pub fn banderas_asignadas(instruccion: &Instruccion, maquina: &Maquina) -> BTreeSet<String> {
    fn recorrer(s: &[Sentencia], maquina: &Maquina, fuera: &mut BTreeSet<String>) {
        for s in s {
            match s {
                Sentencia::Asignar(Destino::Nombre(n), _) if maquina.bandera(n).is_some() => {
                    fuera.insert(n.clone());
                }
                Sentencia::Si(_, cuerpo) => recorrer(cuerpo, maquina, fuera),
                _ => {}
            }
        }
    }
    let mut fuera = BTreeSet::new();
    recorrer(&instruccion.programa.sentencias, maquina, &mut fuera);
    fuera
}

fn mascara(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

fn desplazar_izq(a: i128, n: i128) -> i128 {
    if (0..127).contains(&n) {
        a.wrapping_shl(n as u32)
    } else {
        0
    }
}

fn desplazar_der(a: i128, n: i128) -> i128 {
    match n {
        n if n < 0 => 0,
        n if n >= 127 => {
            if a < 0 {
                -1
            } else {
                0
            }
        }
        n => a >> n,
    }
}

fn binario(op: OpBinario, x: i128, y: i128) -> Result<i128, ErrorRef> {
    let b = |c: bool| i128::from(c);
    Ok(match op {
        OpBinario::Suma => x.wrapping_add(y),
        OpBinario::Resta => x.wrapping_sub(y),
        OpBinario::Producto => x.wrapping_mul(y),
        OpBinario::Division => {
            if y == 0 {
                return Err(ErrorRef::DivisionEntreCero);
            }
            x.wrapping_div(y)
        }
        OpBinario::Modulo => {
            if y == 0 {
                return Err(ErrorRef::DivisionEntreCero);
            }
            x.wrapping_rem(y)
        }
        OpBinario::DesplazaIzq => desplazar_izq(x, y),
        OpBinario::DesplazaDer => desplazar_der(x, y),
        OpBinario::Y => x & y,
        OpBinario::O => x | y,
        OpBinario::Xor => x ^ y,
        OpBinario::Igual => b(x == y),
        OpBinario::Distinto => b(x != y),
        OpBinario::Menor => b(x < y),
        OpBinario::MenorIgual => b(x <= y),
        OpBinario::Mayor => b(x > y),
        OpBinario::MayorIgual => b(x >= y),
        OpBinario::YLogico => b(x != 0 && y != 0),
        OpBinario::OLogico => b(x != 0 || y != 0),
    })
}

/// Ultimo valor asignado a un destino que no es bandera, para las convenciones Z, N y C.
#[derive(Default)]
struct Ultimo {
    valor: Option<(i128, u64, u32)>,
    acarreo: bool,
    explicitas: BTreeSet<String>,
}

/// Evaluador de una instruccion sobre un estado y una memoria.
pub struct Referencia<'a> {
    maquina: &'a Maquina,
    pc: String,
    sp: Option<String>,
    banderas: Option<String>,
}

impl<'a> Referencia<'a> {
    pub fn nueva(maquina: &'a Maquina) -> Self {
        let nombre = |rol| maquina.registro_con_rol(rol).map(|r| r.nombre.clone());
        Referencia {
            maquina,
            pc: nombre(RolRegistro::ContadorPrograma).unwrap_or_default(),
            sp: nombre(RolRegistro::PunteroPila),
            banderas: nombre(RolRegistro::Banderas),
        }
    }

    /// El estado de partida: cada registro en su valor inicial y todas las banderas apagadas.
    pub fn estado_inicial(&self) -> EstadoRef {
        let mut registros = BTreeMap::new();
        for r in &self.maquina.registros {
            if r.rol != RolRegistro::Banderas {
                registros.insert(r.nombre.clone(), r.inicial & mascara(r.bits));
            }
        }
        EstadoRef {
            registros,
            banderas: self
                .maquina
                .banderas
                .iter()
                .map(|b| (b.nombre.clone(), false))
                .collect(),
            ie: false,
            escrituras: BTreeMap::new(),
            lecturas_iniciales: BTreeSet::new(),
            accedidas: BTreeSet::new(),
            salto: false,
            detenida: false,
        }
    }

    /// Valor del registro de banderas armado con las banderas individuales.
    pub fn banderas_empaquetadas(&self, e: &EstadoRef) -> u64 {
        self.maquina
            .banderas
            .iter()
            .filter(|b| e.banderas.get(&b.nombre).copied().unwrap_or(false))
            .fold(0u64, |acc, b| acc | (1u64 << b.bit))
    }

    fn bits_de(&self, nombre: &str) -> Option<u32> {
        self.maquina.registro(nombre).map(|r| r.bits)
    }

    /// Ejecuta la instruccion. `pc_siguiente` es lo que vale `pc` mientras se evalua (D-013);
    /// `memoria` da el contenido de una celda que la instruccion no ha escrito todavia.
    pub fn ejecutar(
        &self,
        instruccion: &Instruccion,
        operandos: &[ValorDeOperando],
        mut estado: EstadoRef,
        pc_siguiente: u64,
        memoria: &dyn AccesoMemoria,
    ) -> Result<ResultadoRef, ErrorRef> {
        let ciclos =
            self.ejecutar_en(instruccion, operandos, &mut estado, pc_siguiente, memoria)?;
        Ok(ResultadoRef { estado, ciclos })
    }

    /// Como [`Referencia::ejecutar`], pero modifica `estado` en el sitio: si la instruccion falla a
    /// medias, `estado` conserva lo que alcanzo a hacer, igual que el emulador.
    pub fn ejecutar_en(
        &self,
        instruccion: &Instruccion,
        operandos: &[ValorDeOperando],
        estado: &mut EstadoRef,
        pc_siguiente: u64,
        memoria: &dyn AccesoMemoria,
    ) -> Result<u64, ErrorRef> {
        if usa_puertos(instruccion) {
            return Err(ErrorRef::UsaPuertos);
        }
        let mascara_pc = mascara(self.bits_de(&self.pc).unwrap_or(64));
        estado
            .registros
            .insert(self.pc.clone(), pc_siguiente & mascara_pc);
        let mut ultimo = Ultimo::default();
        self.sentencias(
            &instruccion.programa.sentencias,
            instruccion,
            operandos,
            estado,
            memoria,
            &mut ultimo,
        )?;
        self.convenciones(instruccion, estado, &ultimo);
        Ok(match instruccion.ciclos {
            Ciclos::Fijo(n) => u64::from(n),
            Ciclos::SegunSalto { tomado, no_tomado } => {
                u64::from(if estado.salto { tomado } else { no_tomado })
            }
        })
    }

    fn convenciones(&self, instruccion: &Instruccion, e: &mut EstadoRef, u: &Ultimo) {
        let Some((sin_truncar, truncado, ancho)) = u.valor else {
            return;
        };
        for nombre in &instruccion.afecta {
            if u.explicitas.contains(nombre) || !e.banderas.contains_key(nombre) {
                continue;
            }
            let encendida = match nombre.as_str() {
                "Z" => truncado == 0,
                "N" => ancho > 0 && (truncado >> (ancho - 1)) & 1 == 1,
                "C" => u.acarreo && (sin_truncar < 0 || (sin_truncar >> ancho) != 0),
                _ => continue,
            };
            e.banderas.insert(nombre.clone(), encendida);
        }
    }

    fn sentencias(
        &self,
        lista: &[Sentencia],
        ins: &Instruccion,
        ops: &[ValorDeOperando],
        e: &mut EstadoRef,
        mem: &dyn AccesoMemoria,
        u: &mut Ultimo,
    ) -> Result<(), ErrorRef> {
        for s in lista {
            match s {
                Sentencia::Detener => {
                    e.detenida = true;
                    return Ok(());
                }
                Sentencia::Si(c, cuerpo) => {
                    if self.expr(c, ins, ops, e, mem)? != 0 {
                        self.sentencias(cuerpo, ins, ops, e, mem, u)?;
                        if e.detenida {
                            return Ok(());
                        }
                    }
                }
                Sentencia::Asignar(d, x) => {
                    let valor = self.expr(x, ins, ops, e, mem)?;
                    let acarreo =
                        matches!(x, Expr::Binario(op, _, _) if op.puede_producir_acarreo());
                    let era_bandera = self.asignar(d, valor, ins, ops, e, mem, u)?;
                    if !era_bandera {
                        u.acarreo = acarreo;
                    }
                }
            }
        }
        Ok(())
    }

    fn operando_registro(&self, ins: &Instruccion, nombre: &str, indice: i128) -> Option<String> {
        let o = ins.operandos.iter().find(|o| o.nombre == nombre)?;
        match &o.tipo {
            TipoOperando::Registro { permitidos } => permitidos.get(indice as usize).cloned(),
            _ => None,
        }
    }

    fn leer_registro(&self, nombre: &str, e: &EstadoRef) -> Option<i128> {
        if self.banderas.as_deref() == Some(nombre) {
            return Some(i128::from(self.banderas_empaquetadas(e)));
        }
        e.registros.get(nombre).map(|v| i128::from(*v))
    }

    fn nombre(
        &self,
        n: &str,
        ins: &Instruccion,
        ops: &[ValorDeOperando],
        e: &EstadoRef,
    ) -> Result<i128, ErrorRef> {
        if let Some(o) = ops.iter().find(|o| o.nombre == n) {
            return match self.operando_registro(ins, n, o.valor) {
                Some(r) => self
                    .leer_registro(&r, e)
                    .ok_or(ErrorRef::NombreDesconocido(r)),
                None => Ok(o.valor),
            };
        }
        if let Some(v) = self.leer_registro(n, e) {
            return Ok(v);
        }
        if let Some(b) = e.banderas.get(n) {
            return Ok(i128::from(*b));
        }
        match n {
            "pc" => self.leer_registro(&self.pc, e),
            "sp" => self.sp.as_deref().and_then(|s| self.leer_registro(s, e)),
            "ie" => Some(i128::from(e.ie)),
            _ => None,
        }
        .ok_or_else(|| ErrorRef::NombreDesconocido(n.to_string()))
    }

    fn expr(
        &self,
        x: &Expr,
        ins: &Instruccion,
        ops: &[ValorDeOperando],
        e: &mut EstadoRef,
        mem: &dyn AccesoMemoria,
    ) -> Result<i128, ErrorRef> {
        Ok(match x {
            Expr::Literal(n) => i128::from(*n),
            Expr::Nombre(n) => self.nombre(n, ins, ops, e)?,
            Expr::Mem(a) => {
                let d = self.direccion(self.expr(a, ins, ops, e, mem)?);
                e.accedidas.insert(d);
                if let Some(v) = e.escrituras.get(&d) {
                    i128::from(*v)
                } else {
                    e.lecturas_iniciales.insert(d);
                    i128::from(mem.leer(d)? & mascara(self.maquina.bits_palabra))
                }
            }
            Expr::Entrada(_) => return Err(ErrorRef::UsaPuertos),
            Expr::Unario(op, a) => {
                let v = self.expr(a, ins, ops, e, mem)?;
                match op {
                    OpUnario::NegacionLogica => i128::from(v == 0),
                    OpUnario::NegacionBits => !v,
                    OpUnario::Menos => v.wrapping_neg(),
                }
            }
            Expr::Binario(op, a, b) => {
                let x = self.expr(a, ins, ops, e, mem)?;
                // El lado derecho de && y || no se evalua si el izquierdo ya decide.
                match op {
                    OpBinario::YLogico if x == 0 => return Ok(0),
                    OpBinario::OLogico if x != 0 => return Ok(1),
                    _ => {}
                }
                let y = self.expr(b, ins, ops, e, mem)?;
                binario(*op, x, y)?
            }
        })
    }

    fn direccion(&self, v: i128) -> u64 {
        (v as u64) & mascara(self.maquina.bits_direccion)
    }

    /// Devuelve `true` si el destino era una bandera.
    #[allow(clippy::too_many_arguments)]
    fn asignar(
        &self,
        d: &Destino,
        valor: i128,
        ins: &Instruccion,
        ops: &[ValorDeOperando],
        e: &mut EstadoRef,
        mem: &dyn AccesoMemoria,
        u: &mut Ultimo,
    ) -> Result<bool, ErrorRef> {
        match d {
            Destino::Nombre(n) => {
                if let Some(o) = ops.iter().find(|o| &o.nombre == n) {
                    let r = self
                        .operando_registro(ins, n, o.valor)
                        .ok_or_else(|| ErrorRef::DestinoNoAsignable(n.clone()))?;
                    self.asignar_registro(&r, valor, e, u)?;
                    return Ok(false);
                }
                if self.maquina.registro(n).is_some() {
                    self.asignar_registro(n, valor, e, u)?;
                    return Ok(false);
                }
                if e.banderas.contains_key(n) {
                    e.banderas.insert(n.clone(), valor != 0);
                    u.explicitas.insert(n.clone());
                    return Ok(true);
                }
                match n.as_str() {
                    "ie" => {
                        e.ie = valor != 0;
                        Ok(false)
                    }
                    "pc" => {
                        let r = self.pc.clone();
                        self.asignar_registro(&r, valor, e, u)?;
                        Ok(false)
                    }
                    "sp" => {
                        let r = self
                            .sp
                            .clone()
                            .ok_or_else(|| ErrorRef::DestinoNoAsignable(n.clone()))?;
                        self.asignar_registro(&r, valor, e, u)?;
                        Ok(false)
                    }
                    _ => Err(ErrorRef::DestinoNoAsignable(n.clone())),
                }
            }
            Destino::Mem(a) => {
                let dir = self.direccion(self.expr(a, ins, ops, e, mem)?);
                let m = mascara(self.maquina.bits_palabra);
                let t = (valor as u64) & m;
                mem.comprobar_escritura(dir)?;
                e.accedidas.insert(dir);
                e.escrituras.insert(dir, t);
                u.valor = Some((valor, t, m.count_ones()));
                Ok(false)
            }
            Destino::Salida(_) => Err(ErrorRef::UsaPuertos),
        }
    }

    fn asignar_registro(
        &self,
        nombre: &str,
        valor: i128,
        e: &mut EstadoRef,
        u: &mut Ultimo,
    ) -> Result<(), ErrorRef> {
        let r = self
            .maquina
            .registro(nombre)
            .ok_or_else(|| ErrorRef::NombreDesconocido(nombre.to_string()))?;
        if r.rol == RolRegistro::Fijo {
            return Err(ErrorRef::DestinoNoAsignable(nombre.to_string()));
        }
        let m = mascara(r.bits);
        let t = (valor as u64) & m;
        if r.rol == RolRegistro::Banderas {
            for b in &self.maquina.banderas {
                e.banderas.insert(b.nombre.clone(), (t >> b.bit) & 1 == 1);
            }
        } else {
            e.registros.insert(nombre.to_string(), t);
        }
        if r.rol == RolRegistro::ContadorPrograma {
            e.salto = true;
        }
        u.valor = Some((valor, t, m.count_ones()));
        Ok(())
    }
}
