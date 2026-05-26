//! Exportacion de una maquina a Verilog (RF-ECO-034, RF-ECO-035).
//!
//! Genera un modelo de simulacion que ejecuta una instruccion por ciclo de reloj, con la misma
//! semantica que el emulador: el contador de programa que ve la semantica ya apunta a la
//! instruccion siguiente, y las banderas de `afecta` siguen las convenciones Z, N y C. No es un
//! diseno sintetizable: las asignaciones son bloqueantes y la memoria es un arreglo.
//!
//! Lo que no modela se dice en [`Modelo::avisos`]: interrupciones, dispositivos, regiones de solo
//! lectura, `in[]`/`out[]` y los pares y bancos de registros (estos ultimos se rechazan).

use std::collections::BTreeSet;
use std::fmt::Write;

use crate::modelo::{Endianness, Instruccion, Maquina, RolRegistro, TipoOperando};
use crate::semantica::ast::{Destino, Expr, OpBinario, OpUnario, Sentencia};

/// El resultado de exportar una maquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modelo {
    /// El modulo `<nombre>_cpu`.
    pub cpu: String,
    /// Un banco de pruebas que carga `programa.mem`, corre hasta `HLT` y escribe los registros.
    pub banco: String,
    /// Lo que el modelo no representa.
    pub avisos: Vec<String>,
}

/// Nombre valido de Verilog a partir del de la maquina.
pub fn identificador(nombre: &str) -> String {
    let mut s: String = nombre
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, 'm');
    }
    s
}

struct Contexto<'a> {
    m: &'a Maquina,
    instr: &'a Instruccion,
}

impl Contexto<'_> {
    fn pc(&self) -> Option<&str> {
        self.m
            .registro_con_rol(RolRegistro::ContadorPrograma)
            .map(|r| r.nombre.as_str())
    }

    fn registro_pila(&self) -> Option<&str> {
        self.m
            .registro_con_rol(RolRegistro::PunteroPila)
            .map(|r| r.nombre.as_str())
    }

    fn nombre(&self, n: &str) -> Result<String, String> {
        if self.instr.operandos.iter().any(|o| o.nombre == n) {
            return Ok(format!("v_{n}"));
        }
        if n == "pc" || Some(n) == self.pc() {
            return Ok("pcn".into());
        }
        if n == "sp" {
            return self
                .registro_pila()
                .map(String::from)
                .ok_or_else(|| "usa `sp` sin registro de pila".into());
        }
        if n == "ie" {
            return Ok("ie".into());
        }
        if let Some(b) = self.m.bandera(n) {
            let f = self.registro_banderas()?;
            return Ok(format!("{f}[{}]", b.bit));
        }
        if self.m.registro(n).is_some() {
            return Ok(n.to_string());
        }
        Err(format!(
            "la semantica usa `{n}`, que no es un operando, un registro ni una bandera"
        ))
    }

    fn registro_banderas(&self) -> Result<&str, String> {
        self.m
            .registro_con_rol(RolRegistro::Banderas)
            .map(|r| r.nombre.as_str())
            .ok_or_else(|| "hay banderas pero ningun registro con el rol de banderas".into())
    }

    fn expr(&self, e: &Expr) -> Result<String, String> {
        Ok(match e {
            Expr::Literal(n) => format!("65'd{n}"),
            Expr::Nombre(n) => self.nombre(n)?,
            Expr::Mem(d) => format!("mem[({}) & ADDR_MASK]", self.expr(d)?),
            Expr::Entrada(_) => "65'd0".into(),
            Expr::Unario(op, x) => {
                let x = self.expr(x)?;
                match op {
                    OpUnario::NegacionLogica => format!("(({x}) == 65'd0)"),
                    OpUnario::NegacionBits => format!("(~({x}))"),
                    OpUnario::Menos => format!("(-({x}))"),
                }
            }
            Expr::Binario(op, a, b) => {
                let (a, b) = (self.expr(a)?, self.expr(b)?);
                match op {
                    OpBinario::YLogico => format!("((({a}) != 65'd0) && (({b}) != 65'd0))"),
                    OpBinario::OLogico => format!("((({a}) != 65'd0) || (({b}) != 65'd0))"),
                    otro => format!("(({a}) {} ({b}))", otro.simbolo()),
                }
            }
        })
    }

    /// Escribe `valor` (un `reg [64:0]` ya calculado) en el destino.
    fn guardar(&self, d: &Destino, o: &mut String, sangria: &str) -> Result<(), String> {
        match d {
            Destino::Mem(dir) => {
                writeln!(
                    o,
                    "{sangria}mem[({}) & ADDR_MASK] = tmp[{}:0];",
                    self.expr(dir)?,
                    self.m.bits_palabra - 1
                )
                .unwrap();
            }
            Destino::Salida(_) => {
                writeln!(o, "{sangria}// out[]: no hay puertos en el modelo").unwrap()
            }
            Destino::Nombre(n) => {
                if let Some(op) = self.instr.operandos.iter().find(|op| op.nombre == *n) {
                    let TipoOperando::Registro { permitidos } = &op.tipo else {
                        return Err(format!("`{n}` no es un registro y no se puede asignar"));
                    };
                    writeln!(o, "{sangria}case (o_{n})").unwrap();
                    for (i, r) in permitidos.iter().enumerate() {
                        let bits = self.m.registro(r).map_or(self.m.bits_palabra, |r| r.bits);
                        writeln!(o, "{sangria}  {i}: {r} = tmp[{}:0];", bits - 1).unwrap();
                    }
                    writeln!(o, "{sangria}  default: ;").unwrap();
                    writeln!(o, "{sangria}endcase").unwrap();
                } else if let Some(b) = self.m.bandera(n) {
                    writeln!(
                        o,
                        "{sangria}{}[{}] = |tmp;",
                        self.registro_banderas()?,
                        b.bit
                    )
                    .unwrap();
                } else {
                    let destino = self.nombre(n)?;
                    let bits = if destino == "pcn" {
                        self.m.bits_direccion
                    } else if destino == "ie" {
                        1
                    } else {
                        self.m
                            .registro(&destino)
                            .map_or(self.m.bits_palabra, |r| r.bits)
                    };
                    writeln!(o, "{sangria}{destino} = tmp[{}:0];", bits - 1).unwrap();
                }
            }
        }
        Ok(())
    }

    fn sentencias(
        &self,
        ss: &[Sentencia],
        o: &mut String,
        nivel: usize,
        afecta: bool,
    ) -> Result<(), String> {
        let sangria = "    ".repeat(nivel);
        for s in ss {
            match s {
                Sentencia::Detener => writeln!(o, "{sangria}detenida = 1'b1;").unwrap(),
                Sentencia::Si(c, cuerpo) => {
                    writeln!(o, "{sangria}if ({} != 65'd0) begin", self.expr(c)?).unwrap();
                    self.sentencias(cuerpo, o, nivel + 1, afecta)?;
                    writeln!(o, "{sangria}end").unwrap();
                }
                Sentencia::Asignar(d, e) => {
                    writeln!(o, "{sangria}tmp = {};", self.expr(e)?).unwrap();
                    let es_bandera = matches!(d, Destino::Nombre(n) if self.m.bandera(n).is_some());
                    if afecta && !es_bandera {
                        let acarreo =
                            matches!(e, Expr::Binario(op, _, _) if op.puede_producir_acarreo());
                        let ancho = match d {
                            Destino::Nombre(n)
                                if self.instr.operandos.iter().any(|o| o.nombre == *n) =>
                            {
                                match &self
                                    .instr
                                    .operandos
                                    .iter()
                                    .find(|o| o.nombre == *n)
                                    .unwrap()
                                    .tipo
                                {
                                    TipoOperando::Registro { permitidos } => permitidos
                                        .first()
                                        .and_then(|r| self.m.registro(r))
                                        .map_or(self.m.bits_palabra, |r| r.bits),
                                    _ => self.m.bits_palabra,
                                }
                            }
                            Destino::Nombre(n) => {
                                self.m.registro(n).map_or(self.m.bits_palabra, |r| r.bits)
                            }
                            _ => self.m.bits_palabra,
                        };
                        writeln!(
                            o,
                            "{sangria}ult = tmp; ult_ancho = {ancho}; ult_acarreo = 1'b{};",
                            u8::from(acarreo)
                        )
                        .unwrap();
                    }
                    self.guardar(d, o, &sangria)?;
                }
            }
        }
        Ok(())
    }
}

/// Valor de un operando como un `reg [64:0]` a partir de las palabras `w0`, `w1`...
fn extraer(m: &Maquina, i: &Instruccion, nombre: &str) -> Result<String, String> {
    let mut frag = i.codificacion.fragmentos(nombre);
    if frag.is_empty() {
        return Err(format!(
            "{}: el operando `{nombre}` no esta en la codificacion",
            i.mnemonico
        ));
    }
    // Concatenacion de mas a menos significativo.
    if m.endianness == Endianness::Little {
        frag.reverse();
    }
    let partes: Vec<String> = frag
        .iter()
        .map(|f| {
            format!(
                "w{}[{}:{}]",
                f.palabra,
                f.desplazamiento + f.bits - 1,
                f.desplazamiento
            )
        })
        .collect();
    Ok(format!("{{{}}}", partes.join(", ")))
}

fn instruccion_a_verilog(m: &Maquina, i: &Instruccion, o: &mut String) -> Result<(), String> {
    let palabras = i.codificacion.palabras();
    let p0 = &palabras[0];
    writeln!(o, "      // {}", i.sintaxis).unwrap();
    writeln!(
        o,
        "      if ((w0 & {}'h{:X}) == {}'h{:X}) begin",
        m.bits_palabra,
        p0.mascara(),
        m.bits_palabra,
        p0.valor_fijo()
    )
    .unwrap();
    writeln!(o, "        pcn = pc + {};", palabras.len()).unwrap();
    for op in &i.operandos {
        let bruto = extraer(m, i, &op.nombre)?;
        let bits = i.codificacion.bits_totales(&op.nombre);
        let n = &op.nombre;
        match &op.tipo {
            TipoOperando::Registro { permitidos } => {
                writeln!(o, "        o_{n} = {bruto};").unwrap();
                let mut cadena = String::new();
                for (k, r) in permitidos.iter().enumerate() {
                    write!(cadena, "(o_{n} == {k}) ? {r} : ").unwrap();
                }
                writeln!(o, "        v_{n} = {cadena}65'd0;").unwrap();
            }
            TipoOperando::Inmediato { signo: true, .. } | TipoOperando::Desplazamiento => {
                writeln!(o, "        o_{n} = {bruto};").unwrap();
                writeln!(
                    o,
                    "        v_{n} = {{{{{}{{o_{n}[{}]}}}}, o_{n}[{}:0]}};",
                    65 - bits,
                    bits - 1,
                    bits - 1
                )
                .unwrap();
            }
            _ => {
                writeln!(o, "        o_{n} = {bruto};").unwrap();
                writeln!(o, "        v_{n} = o_{n};").unwrap();
            }
        }
    }
    let c = Contexto { m, instr: i };
    let afecta = !i.afecta.is_empty();
    if afecta {
        writeln!(
            o,
            "        ult_ancho = 0; ult_acarreo = 1'b0; ult = 65'd0; explicitas = 1'b0;"
        )
        .unwrap();
    }
    c.sentencias(&i.programa.sentencias, o, 4, afecta)?;
    if afecta {
        let f = c.registro_banderas()?;
        writeln!(o, "        if (ult_ancho != 0) begin").unwrap();
        for nombre in &i.afecta {
            let Some(b) = m.bandera(nombre) else { continue };
            let expr = match nombre.as_str() {
                "Z" => "((ult & ((65'd1 << ult_ancho) - 65'd1)) == 65'd0)".to_string(),
                "N" => "ult[ult_ancho - 1]".to_string(),
                "C" => "(ult_acarreo && ((ult >> ult_ancho) != 65'd0))".to_string(),
                _ => continue,
            };
            writeln!(o, "          {f}[{}] = {expr};", b.bit).unwrap();
        }
        writeln!(o, "        end").unwrap();
    }
    writeln!(o, "      end else").unwrap();
    Ok(())
}

/// Genera el modelo en Verilog de una maquina.
pub fn exportar(m: &Maquina) -> Result<Modelo, String> {
    if !m.pares.is_empty() || !m.bancos_registros.is_empty() {
        return Err(
            "las maquinas con pares o bancos de registros no se pueden exportar a Verilog".into(),
        );
    }
    let pc = m
        .registro_con_rol(RolRegistro::ContadorPrograma)
        .ok_or("la maquina no tiene contador de programa")?;
    let id = identificador(&m.nombre);
    let mut avisos = Vec::new();
    if m.interrupciones.is_some() || m.instrucciones.iter().any(|i| i.mnemonico == "RETI") {
        avisos.push("las interrupciones no se modelan".to_string());
    }
    if !m.dispositivos.is_empty() {
        avisos.push("los dispositivos no se modelan; in[] da 0 y out[] se ignora".to_string());
    }
    if m.regiones.iter().any(|r| !r.acceso.escritura) {
        avisos.push("las regiones de solo lectura se tratan como memoria normal".to_string());
    }
    if m.instrucciones.iter().any(|i| {
        i.codificacion
            .palabras()
            .iter()
            .any(|p| p.ancho() != m.bits_palabra)
    }) {
        return Err(
            "hay instrucciones cuyas palabras no miden lo mismo que la palabra de la maquina"
                .into(),
        );
    }

    let mut o = String::new();
    writeln!(
        o,
        "// Modelo de simulacion de {}. {}",
        m.nombre,
        crate::marca_generado("lorenz exportar verilog")
    )
    .unwrap();
    writeln!(
        o,
        "// Una instruccion por ciclo de reloj. No es sintetizable tal cual."
    )
    .unwrap();
    for a in &avisos {
        writeln!(o, "// Aviso: {a}.").unwrap();
    }
    writeln!(
        o,
        "`timescale 1ns/1ps\nmodule {id}_cpu (input clk, input rst, output reg detenida);"
    )
    .unwrap();
    writeln!(
        o,
        "  localparam [64:0] ADDR_MASK = (65'd1 << {}) - 65'd1;",
        m.bits_direccion
    )
    .unwrap();
    writeln!(
        o,
        "  reg [{}:0] mem [0:{}];",
        m.bits_palabra - 1,
        (1u64 << m.bits_direccion.min(24)) - 1
    )
    .unwrap();
    for r in &m.registros {
        if r.rol == RolRegistro::ContadorPrograma {
            writeln!(o, "  reg [{}:0] pc;", r.bits - 1).unwrap();
        } else {
            writeln!(o, "  reg [{}:0] {};", r.bits - 1, r.nombre).unwrap();
        }
    }
    writeln!(o, "  reg [{}:0] pcn;\n  reg ie;", pc.bits - 1).unwrap();
    writeln!(o, "  reg [{}:0] w0, w1, w2, w3;\n  reg [64:0] tmp, ult;\n  reg [6:0] ult_ancho;\n  reg ult_acarreo, explicitas;", m.bits_palabra - 1).unwrap();
    let mut nombres: BTreeSet<&str> = BTreeSet::new();
    for i in &m.instrucciones {
        for op in &i.operandos {
            nombres.insert(&op.nombre);
        }
    }
    for n in &nombres {
        writeln!(o, "  reg [64:0] o_{n}, v_{n};").unwrap();
    }
    writeln!(o, "\n  always @(posedge clk) begin\n    if (rst) begin").unwrap();
    for r in &m.registros {
        let destino = if r.rol == RolRegistro::ContadorPrograma {
            "pc".to_string()
        } else {
            r.nombre.clone()
        };
        let inicial = if r.rol == RolRegistro::ContadorPrograma {
            m.arranque
        } else {
            r.inicial
        };
        writeln!(o, "      {destino} <= {}'d{inicial};", r.bits).unwrap();
    }
    writeln!(
        o,
        "      ie <= 1'b0;\n      detenida <= 1'b0;\n    end else if (!detenida) begin"
    )
    .unwrap();
    writeln!(o, "      w0 = mem[pc & ADDR_MASK];\n      w1 = mem[(pc + 1) & ADDR_MASK];\n      w2 = mem[(pc + 2) & ADDR_MASK];\n      w3 = mem[(pc + 3) & ADDR_MASK];").unwrap();
    for i in &m.instrucciones {
        if i.codificacion.palabras().len() > 4 {
            return Err(format!("{}: ocupa mas de cuatro palabras", i.mnemonico));
        }
        instruccion_a_verilog(m, i, &mut o).map_err(|e| format!("{}: {e}", i.mnemonico))?;
    }
    writeln!(o, "      begin\n        // Codigo de operacion desconocido: se detiene.\n        pcn = pc;\n        detenida = 1'b1;\n      end").unwrap();
    // El PC se actualiza al final; los registros ya se actualizaron con asignaciones bloqueantes.
    writeln!(o, "      pc <= pcn;\n    end\n  end\nendmodule").unwrap();

    let mut b = String::new();
    writeln!(b, "// Banco de pruebas de {id}_cpu: carga programa.mem y corre hasta que la maquina se detiene.\n// {}", crate::marca_generado("lorenz exportar verilog")).unwrap();
    writeln!(b, "`timescale 1ns/1ps\nmodule {id}_tb;\n  reg clk = 0;\n  reg rst = 1;\n  wire detenida;\n  integer ciclos = 0;").unwrap();
    writeln!(
        b,
        "  {id}_cpu dut (.clk(clk), .rst(rst), .detenida(detenida));\n  always #5 clk = ~clk;"
    )
    .unwrap();
    writeln!(b, "  initial begin\n    $readmemh(\"programa.mem\", dut.mem);\n    #12 rst = 0;\n    while (!detenida && ciclos < 100000) begin\n      @(posedge clk);\n      #1 ciclos = ciclos + 1;\n    end\n    #1;").unwrap();
    writeln!(b, "    $display(\"detenida: %0s\", detenida ? \"si\" : \"no\");\n    $display(\"instrucciones: %0d\", ciclos);").unwrap();
    for r in &m.registros {
        let n = if r.rol == RolRegistro::ContadorPrograma {
            "pc".to_string()
        } else {
            r.nombre.clone()
        };
        writeln!(b, "    $display(\"{}=0x%0h\", dut.{n});", r.nombre).unwrap();
    }
    writeln!(b, "    $finish;\n  end\nendmodule").unwrap();

    Ok(Modelo {
        cpu: o,
        banco: b,
        avisos,
    })
}

/// Contenido de `programa.mem` para `$readmemh`: una linea `@direccion` por tramo.
pub fn imagen_mem(tramos: &[(u64, Vec<u64>)], bits_palabra: u32) -> String {
    let digitos = (bits_palabra as usize).div_ceil(4);
    let mut s = String::from("// programa para $readmemh\n");
    for (dir, palabras) in tramos {
        writeln!(s, "@{dir:X}").unwrap();
        for p in palabras {
            writeln!(s, "{p:0digitos$X}").unwrap();
        }
    }
    s
}
