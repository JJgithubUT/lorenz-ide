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

// ---------------------------------------------------------------------------
// precedencia y asociatividad
// ---------------------------------------------------------------------------

#[test]
fn el_producto_liga_mas_que_la_suma() {
    assert_eq!(
        analizar_expresion("a + b * c").unwrap(),
        bin(OpBinario::Suma, n("a"), bin(OpBinario::Producto, n("b"), n("c")))
    );
}

#[test]
fn la_suma_es_asociativa_a_la_izquierda() {
    assert_eq!(
        analizar_expresion("a - b - c").unwrap(),
        bin(OpBinario::Resta, bin(OpBinario::Resta, n("a"), n("b")), n("c"))
    );
}

#[test]
fn los_parentesis_cambian_la_agrupacion() {
    assert_eq!(
        analizar_expresion("(a + b) * c").unwrap(),
        bin(OpBinario::Producto, bin(OpBinario::Suma, n("a"), n("b")), n("c"))
    );
}

#[test]
fn el_desplazamiento_liga_menos_que_la_suma() {
    assert_eq!(
        analizar_expresion("a << b + 1").unwrap(),
        bin(OpBinario::DesplazaIzq, n("a"), bin(OpBinario::Suma, n("b"), lit(1)))
    );
}

#[test]
fn el_and_de_bits_liga_menos_que_el_desplazamiento() {
    assert_eq!(
        analizar_expresion("a & b << 1").unwrap(),
        bin(OpBinario::Y, n("a"), bin(OpBinario::DesplazaIzq, n("b"), lit(1)))
    );
}

#[test]
fn xor_liga_menos_que_and_y_mas_que_or() {
    assert_eq!(
        analizar_expresion("a | b ^ c & d").unwrap(),
        bin(
            OpBinario::O,
            n("a"),
            bin(OpBinario::Xor, n("b"), bin(OpBinario::Y, n("c"), n("d")))
        )
    );
}

#[test]
fn la_comparacion_liga_menos_que_las_operaciones_de_bits() {
    // como en Rust: a == b | c se lee a == (b | c)
    assert_eq!(
        analizar_expresion("a == b | c").unwrap(),
        bin(OpBinario::Igual, n("a"), bin(OpBinario::O, n("b"), n("c")))
    );
}

#[test]
fn el_y_logico_liga_menos_que_la_comparacion() {
    assert_eq!(
        analizar_expresion("a < b && c != d").unwrap(),
        bin(
            OpBinario::YLogico,
            bin(OpBinario::Menor, n("a"), n("b")),
            bin(OpBinario::Distinto, n("c"), n("d"))
        )
    );
}

#[test]
fn el_o_logico_es_el_de_menor_precedencia() {
    assert_eq!(
        analizar_expresion("a || b && c").unwrap(),
        bin(OpBinario::OLogico, n("a"), bin(OpBinario::YLogico, n("b"), n("c")))
    );
}

#[test]
fn reconoce_los_seis_operadores_de_comparacion() {
    let casos = [
        ("a == b", OpBinario::Igual),
        ("a != b", OpBinario::Distinto),
        ("a < b", OpBinario::Menor),
        ("a <= b", OpBinario::MenorIgual),
        ("a > b", OpBinario::Mayor),
        ("a >= b", OpBinario::MayorIgual),
    ];
    for (texto, op) in casos {
        assert_eq!(analizar_expresion(texto).unwrap(), bin(op, n("a"), n("b")), "{texto}");
    }
}

#[test]
fn reconoce_division_y_modulo() {
    assert_eq!(analizar_expresion("a / b").unwrap(), bin(OpBinario::Division, n("a"), n("b")));
    assert_eq!(analizar_expresion("a % b").unwrap(), bin(OpBinario::Modulo, n("a"), n("b")));
}

#[test]
fn reconoce_el_desplazamiento_a_la_derecha() {
    assert_eq!(analizar_expresion("a >> 2").unwrap(), bin(OpBinario::DesplazaDer, n("a"), lit(2)));
}
