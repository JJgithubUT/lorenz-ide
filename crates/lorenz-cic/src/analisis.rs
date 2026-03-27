//! Que registros lee y escribe una instruccion y si es un salto, a partir de su semantica.

use lorenz_arch::modelo::{Instruccion, Maquina, RolRegistro};
use lorenz_arch::semantica::{Destino, Expr, Sentencia};
use lorenz_emu::OperandoDecodificado;

/// Dependencias de una instruccion con sus operandos ya decodificados.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dependencias {
    pub lee: Vec<String>,
    pub escribe: Vec<String>,
    pub es_salto: bool,
    pub incondicional: bool,
    pub indirecto: bool,
}

struct Resolutor<'a> {
    maquina: &'a Maquina,
    operandos: &'a [OperandoDecodificado],
}

impl Resolutor<'_> {
    /// Nombres de registros o banderas a los que se refiere un nombre de la semantica. Un par
    /// cuenta como sus dos registros.
    fn nombres(&self, n: &str) -> Vec<String> {
        if let Some(o) = self.operandos.iter().find(|o| o.nombre == n) {
            return match &o.registro {
                Some(r) => self.expandir(r),
                None => Vec::new(),
            };
        }
        match n {
            "pc" => Vec::new(),
            "sp" => self
                .maquina
                .registro_con_rol(RolRegistro::PunteroPila)
                .map(|r| vec![r.nombre.clone()])
                .unwrap_or_default(),
            _ => self.expandir(n),
        }
    }

    fn expandir(&self, n: &str) -> Vec<String> {
        if let Some(p) = self.maquina.par(n) {
            return vec![p.alto.clone(), p.bajo.clone()];
        }
        // Igual que el interprete: primero registros, despues banderas. Una bandera lleva el
        // prefijo `bandera ` para no confundirla con un registro que se llame igual.
        if self.maquina.registro(n).is_some() || n == "ie" {
            return vec![n.to_string()];
        }
        if self.maquina.bandera(n).is_some() {
            return vec![format!("bandera {n}")];
        }
        Vec::new()
    }

    fn es_pc(&self, n: &str) -> bool {
        n == "pc"
            || self
                .maquina
                .registro_con_rol(RolRegistro::ContadorPrograma)
                .is_some_and(|r| r.nombre == n)
    }

    fn lecturas(&self, e: &Expr, fuera: &mut Vec<String>, memoria: &mut bool) {
        match e {
            Expr::Literal(_) => {}
            Expr::Nombre(n) => fuera.extend(self.nombres(n)),
            Expr::Mem(a) | Expr::Entrada(a) => {
                *memoria |= matches!(e, Expr::Mem(_));
                self.lecturas(a, fuera, memoria);
            }
            Expr::Unario(_, a) => self.lecturas(a, fuera, memoria),
            Expr::Binario(_, a, b) => {
                self.lecturas(a, fuera, memoria);
                self.lecturas(b, fuera, memoria);
            }
        }
    }

    /// Recorre las sentencias. `condicional` dice si ya se esta dentro de un `si`.
    fn recorrer(&self, s: &[Sentencia], condicional: bool, d: &mut Dependencias) {
        for x in s {
            match x {
                Sentencia::Asignar(dest, valor) => {
                    let mut mem = false;
                    self.lecturas(valor, &mut d.lee, &mut mem);
                    match dest {
                        Destino::Nombre(n) if self.es_pc(n) => {
                            d.es_salto = true;
                            if !condicional {
                                d.incondicional = true;
                            }
                            let mut v = Vec::new();
                            self.lecturas(valor, &mut v, &mut mem);
                            if mem || !v.is_empty() {
                                d.indirecto = true;
                            }
                        }
                        Destino::Nombre(n) => d.escribe.extend(self.nombres(n)),
                        Destino::Mem(a) | Destino::Salida(a) => {
                            self.lecturas(a, &mut d.lee, &mut mem)
                        }
                    }
                }
                Sentencia::Si(c, cuerpo) => {
                    let mut mem = false;
                    self.lecturas(c, &mut d.lee, &mut mem);
                    self.recorrer(cuerpo, true, d);
                }
                Sentencia::Detener => {}
            }
        }
    }
}

/// Calcula las dependencias de una instruccion con los operandos que tiene en este momento.
pub fn dependencias(
    maquina: &Maquina,
    instruccion: &Instruccion,
    operandos: &[OperandoDecodificado],
) -> Dependencias {
    let r = Resolutor { maquina, operandos };
    let mut d = Dependencias::default();
    r.recorrer(&instruccion.programa.sentencias, false, &mut d);
    d.escribe
        .extend(instruccion.afecta.iter().map(|b| format!("bandera {b}")));
    for v in [&mut d.lee, &mut d.escribe] {
        v.sort();
        v.dedup();
    }
    d
}
