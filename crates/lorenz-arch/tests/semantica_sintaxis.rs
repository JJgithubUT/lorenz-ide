//! Pruebas del analizador sintactico de la semantica (RF-ARQ-022).

use lorenz_arch::semantica::{
    analizar, analizar_expresion, Destino, Expr, OpBinario, OpUnario, Sentencia,
};

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
    assert_eq!(
        analizar_expresion("mem[dir]").unwrap(),
        Expr::Mem(Box::new(n("dir")))
    );
}

#[test]
fn analiza_una_entrada() {
    assert_eq!(
        analizar_expresion("in[puerto]").unwrap(),
        Expr::Entrada(Box::new(n("puerto")))
    );
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
        bin(
            OpBinario::Suma,
            n("a"),
            bin(OpBinario::Producto, n("b"), n("c"))
        )
    );
}

#[test]
fn la_suma_es_asociativa_a_la_izquierda() {
    assert_eq!(
        analizar_expresion("a - b - c").unwrap(),
        bin(
            OpBinario::Resta,
            bin(OpBinario::Resta, n("a"), n("b")),
            n("c")
        )
    );
}

#[test]
fn los_parentesis_cambian_la_agrupacion() {
    assert_eq!(
        analizar_expresion("(a + b) * c").unwrap(),
        bin(
            OpBinario::Producto,
            bin(OpBinario::Suma, n("a"), n("b")),
            n("c")
        )
    );
}

#[test]
fn el_desplazamiento_liga_menos_que_la_suma() {
    assert_eq!(
        analizar_expresion("a << b + 1").unwrap(),
        bin(
            OpBinario::DesplazaIzq,
            n("a"),
            bin(OpBinario::Suma, n("b"), lit(1))
        )
    );
}

#[test]
fn el_and_de_bits_liga_menos_que_el_desplazamiento() {
    assert_eq!(
        analizar_expresion("a & b << 1").unwrap(),
        bin(
            OpBinario::Y,
            n("a"),
            bin(OpBinario::DesplazaIzq, n("b"), lit(1))
        )
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
        bin(
            OpBinario::OLogico,
            n("a"),
            bin(OpBinario::YLogico, n("b"), n("c"))
        )
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
        assert_eq!(
            analizar_expresion(texto).unwrap(),
            bin(op, n("a"), n("b")),
            "{texto}"
        );
    }
}

#[test]
fn reconoce_division_y_modulo() {
    assert_eq!(
        analizar_expresion("a / b").unwrap(),
        bin(OpBinario::Division, n("a"), n("b"))
    );
    assert_eq!(
        analizar_expresion("a % b").unwrap(),
        bin(OpBinario::Modulo, n("a"), n("b"))
    );
}

#[test]
fn reconoce_el_desplazamiento_a_la_derecha() {
    assert_eq!(
        analizar_expresion("a >> 2").unwrap(),
        bin(OpBinario::DesplazaDer, n("a"), lit(2))
    );
}

// ---------------------------------------------------------------------------
// sentencias
// ---------------------------------------------------------------------------

#[test]
fn un_texto_vacio_es_un_programa_vacio() {
    assert!(analizar("").unwrap().sentencias.is_empty());
    assert!(analizar("   ").unwrap().sentencias.is_empty());
}

#[test]
fn analiza_una_asignacion_a_registro() {
    let p = analizar("rd = rs").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(Destino::Nombre("rd".into()), n("rs"))]
    );
}

#[test]
fn analiza_una_asignacion_con_aritmetica() {
    let p = analizar("rd = rd + rs").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(
            Destino::Nombre("rd".into()),
            bin(OpBinario::Suma, n("rd"), n("rs"))
        )]
    );
}

#[test]
fn analiza_una_escritura_en_memoria() {
    let p = analizar("mem[dir] = rs").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(Destino::Mem(n("dir")), n("rs"))]
    );
}

#[test]
fn analiza_una_lectura_de_memoria() {
    let p = analizar("rd = mem[dir]").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(
            Destino::Nombre("rd".into()),
            Expr::Mem(Box::new(n("dir")))
        )]
    );
}

#[test]
fn analiza_una_salida_a_puerto() {
    let p = analizar("out[puerto] = rs").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(Destino::Salida(n("puerto")), n("rs"))]
    );
}

#[test]
fn analiza_una_entrada_desde_puerto() {
    let p = analizar("rd = in[puerto]").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Asignar(
            Destino::Nombre("rd".into()),
            Expr::Entrada(Box::new(n("puerto")))
        )]
    );
}

#[test]
fn analiza_detener() {
    assert_eq!(
        analizar("detener").unwrap().sentencias,
        vec![Sentencia::Detener]
    );
}

#[test]
fn analiza_un_condicional_simple() {
    let p = analizar("si Z: pc = dir").unwrap();
    assert_eq!(
        p.sentencias,
        vec![Sentencia::Si(
            n("Z"),
            vec![Sentencia::Asignar(Destino::Nombre("pc".into()), n("dir"))]
        )]
    );
}

#[test]
fn analiza_un_condicional_con_condicion_compuesta() {
    let p = analizar("si Z && !C: pc = dir").unwrap();
    match &p.sentencias[0] {
        Sentencia::Si(cond, _) => assert_eq!(
            *cond,
            bin(
                OpBinario::YLogico,
                n("Z"),
                Expr::Unario(OpUnario::NegacionLogica, Box::new(n("C")))
            )
        ),
        otro => panic!("se esperaba un condicional, hay {otro:?}"),
    }
}

#[test]
fn analiza_un_condicional_con_llaves() {
    let p = analizar("si Z: { a = 1; b = 2 }").unwrap();
    match &p.sentencias[0] {
        Sentencia::Si(_, cuerpo) => assert_eq!(cuerpo.len(), 2),
        otro => panic!("se esperaba un condicional, hay {otro:?}"),
    }
}

#[test]
fn analiza_condicionales_anidados() {
    let p = analizar("si a: si b: c = 1").unwrap();
    match &p.sentencias[0] {
        Sentencia::Si(_, cuerpo) => assert!(matches!(cuerpo[0], Sentencia::Si(_, _))),
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn analiza_una_secuencia_con_punto_y_coma() {
    let p = analizar("sp = sp - 1; mem[sp] = rs").unwrap();
    assert_eq!(p.sentencias.len(), 2);
}

#[test]
fn admite_un_punto_y_coma_final() {
    assert_eq!(analizar("a = 1;").unwrap().sentencias.len(), 1);
}

#[test]
fn admite_puntos_y_comas_repetidos() {
    assert_eq!(analizar("a = 1;; b = 2").unwrap().sentencias.len(), 2);
}

#[test]
fn el_condicional_sin_llaves_solo_abarca_una_sentencia() {
    let p = analizar("si Z: a = 1; b = 2").unwrap();
    assert_eq!(p.sentencias.len(), 2);
    assert!(matches!(p.sentencias[1], Sentencia::Asignar(_, _)));
}

#[test]
fn un_condicional_con_llaves_vacias_es_valido() {
    let p = analizar("si Z: { }").unwrap();
    match &p.sentencias[0] {
        Sentencia::Si(_, cuerpo) => assert!(cuerpo.is_empty()),
        otro => panic!("{otro:?}"),
    }
}

// ---------------------------------------------------------------------------
// errores
// ---------------------------------------------------------------------------

#[test]
fn rechaza_una_asignacion_sin_valor() {
    assert!(analizar("rd =").is_err());
}

#[test]
fn rechaza_una_asignacion_sin_igual() {
    assert!(analizar("rd rs").is_err());
}

#[test]
fn rechaza_asignar_a_un_literal() {
    assert!(analizar("5 = rd").is_err());
}

#[test]
fn rechaza_un_condicional_sin_dos_puntos() {
    assert!(analizar("si Z pc = dir").is_err());
}

#[test]
fn rechaza_un_condicional_sin_cuerpo() {
    assert!(analizar("si Z:").is_err());
}

#[test]
fn rechaza_un_parentesis_sin_cerrar() {
    assert!(analizar_expresion("(a + b").is_err());
}

#[test]
fn rechaza_un_corchete_sin_cerrar() {
    assert!(analizar_expresion("mem[a").is_err());
}

#[test]
fn rechaza_una_llave_sin_cerrar() {
    assert!(analizar("si Z: { a = 1").is_err());
}

#[test]
fn rechaza_un_operador_sin_segundo_operando() {
    assert!(analizar_expresion("a +").is_err());
}

#[test]
fn rechaza_dos_valores_seguidos() {
    assert!(analizar_expresion("a b").is_err());
}

#[test]
fn rechaza_una_expresion_donde_va_una_sentencia() {
    assert!(analizar("a + b").is_err());
}

#[test]
fn rechaza_sentencias_sin_separador() {
    assert!(analizar("a = 1 b = 2").is_err());
}

#[test]
fn rechaza_detener_con_argumentos() {
    assert!(analizar("detener 1").is_err());
}

#[test]
fn rechaza_una_palabra_reservada_como_valor() {
    assert!(analizar_expresion("si").is_err());
}

#[test]
fn el_error_indica_la_posicion() {
    let e = analizar("rd = ").unwrap_err();
    assert_eq!(e.posicion, 5);
}

#[test]
fn el_error_dice_que_se_encontro() {
    let e = analizar("rd = ;").unwrap_err();
    assert!(e.mensaje.contains("';'"), "mensaje: {}", e.mensaje);
}

#[test]
fn un_error_lexico_se_propaga() {
    assert!(analizar("rd = rs @ 1").is_err());
}

// ---------------------------------------------------------------------------
// impresion: reproduce un texto que vuelve a analizarse igual
// ---------------------------------------------------------------------------

fn ida_y_vuelta(texto: &str) {
    let primero = analizar(texto).unwrap();
    let impreso = primero.to_string();
    let segundo = analizar(&impreso).unwrap_or_else(|e| panic!("'{impreso}' no se analiza: {e}"));
    assert_eq!(
        primero, segundo,
        "texto original: '{texto}', impreso: '{impreso}'"
    );
}

#[test]
fn la_impresion_reproduce_asignaciones() {
    ida_y_vuelta("rd = rd + rs");
    ida_y_vuelta("mem[dir] = rs");
    ida_y_vuelta("rd = mem[sp + 1]");
}

#[test]
fn la_impresion_reproduce_condicionales() {
    ida_y_vuelta("si Z: pc = dir");
    ida_y_vuelta("si Z && !C: { a = 1; b = 2 }");
}

#[test]
fn la_impresion_reproduce_la_precedencia() {
    ida_y_vuelta("a = 1 + 2 * 3 << 4 & 5 | 6 ^ 7");
    ida_y_vuelta("a = (1 + 2) * 3");
    ida_y_vuelta("a = b < c && d >= e || f == g");
}

#[test]
fn la_impresion_reproduce_secuencias_y_detener() {
    ida_y_vuelta("sp = sp - 1; mem[sp] = rs; detener");
}

#[test]
fn la_impresion_reproduce_entrada_y_salida() {
    ida_y_vuelta("out[p] = in[q]");
}
