//! Pruebas del analizador sintactico de la semantica (RF-ARQ-022).

use lorenz_arch::semantica::{analizar, analizar_expresion, Destino, Expr, OpBinario, OpUnario, Sentencia};

fn n(nombre: &str) -> Expr {
    Expr::Nombre(nombre.to_string())
}

fn lit(v: u64) -> Expr {
    Expr::Literal(v)
}

fn bin(op: OpBinario, a: Expr, b: Expr) -> Expr {
    Expr::Binario(op, Box::new(a), Box::new(b))
}

// ---------------------------------------------------------------------------
// expresiones primarias
// ---------------------------------------------------------------------------

#[test]
fn analiza_un_literal() {
    assert_eq!(analizar_expresion("7").unwrap(), lit(7));
}

#[test]
fn analiza_un_nombre() {
    assert_eq!(analizar_expresion("rd").unwrap(), n("rd"));
}

#[test]
fn analiza_un_acceso_a_memoria() {
    assert_eq!(analizar_expresion("mem[dir]").unwrap(), Expr::Mem(Box::new(n("dir"))));
}

#[test]
fn analiza_una_entrada() {
    assert_eq!(analizar_expresion("in[puerto]").unwrap(), Expr::Entrada(Box::new(n("puerto"))));
}

#[test]
fn analiza_un_acceso_a_memoria_con_expresion() {
    assert_eq!(
        analizar_expresion("mem[sp + 1]").unwrap(),
        Expr::Mem(Box::new(bin(OpBinario::Suma, n("sp"), lit(1))))
    );
}

#[test]
fn analiza_accesos_anidados() {
    assert_eq!(
        analizar_expresion("mem[mem[dir]]").unwrap(),
        Expr::Mem(Box::new(Expr::Mem(Box::new(n("dir")))))
    );
}

#[test]
fn los_parentesis_no_dejan_rastro_en_el_arbol() {
    assert_eq!(analizar_expresion("(rd)").unwrap(), n("rd"));
    assert_eq!(analizar_expresion("((rd))").unwrap(), n("rd"));
}

#[test]
fn mem_sin_corchetes_es_un_nombre() {
    assert_eq!(analizar_expresion("mem").unwrap(), n("mem"));
}

// ---------------------------------------------------------------------------
// operadores unarios
// ---------------------------------------------------------------------------

#[test]
fn analiza_la_negacion_logica() {
    assert_eq!(
        analizar_expresion("!Z").unwrap(),
        Expr::Unario(OpUnario::NegacionLogica, Box::new(n("Z")))
    );
}

#[test]
fn analiza_el_complemento_de_bits() {
    assert_eq!(
        analizar_expresion("~rs").unwrap(),
        Expr::Unario(OpUnario::NegacionBits, Box::new(n("rs")))
    );
}

#[test]
fn analiza_el_cambio_de_signo() {
    assert_eq!(
        analizar_expresion("-rs").unwrap(),
        Expr::Unario(OpUnario::Menos, Box::new(n("rs")))
    );
}

#[test]
fn los_unarios_se_apilan() {
    assert_eq!(
        analizar_expresion("!!Z").unwrap(),
        Expr::Unario(
            OpUnario::NegacionLogica,
            Box::new(Expr::Unario(OpUnario::NegacionLogica, Box::new(n("Z"))))
        )
    );
}

#[test]
fn el_unario_liga_mas_fuerte_que_el_binario() {
    assert_eq!(
        analizar_expresion("-a + b").unwrap(),
        bin(
            OpBinario::Suma,
            Expr::Unario(OpUnario::Menos, Box::new(n("a"))),
            n("b")
        )
    );
}
