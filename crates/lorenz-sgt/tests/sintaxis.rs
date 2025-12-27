//! El analizador sintactico de SGT.

use lorenz_sgt::ast::*;
use lorenz_sgt::sintaxis::{analizar_expresion, analizar_programa};

fn expr(t: &str) -> ClaseExpr {
    analizar_expresion("t.sgt", t).unwrap().clase
}

fn programa(t: &str) -> Programa {
    analizar_programa("t.sgt", t).unwrap_or_else(|e| panic!("{e}"))
}

fn error(t: &str) -> String {
    analizar_programa("t.sgt", t).unwrap_err().to_string()
}

fn funcion(p: &Programa) -> &DefFuncion {
    match &p.elementos[0] {
        Elemento::Funcion(f) => f,
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn la_multiplicacion_liga_mas_que_la_suma() {
    // 1 + (2 * 3)
    match expr("1 + 2 * 3") {
        ClaseExpr::Binario(OpBinario::Suma, a, b) => {
            assert_eq!(a.clase, ClaseExpr::Entero(1));
            assert!(matches!(b.clase, ClaseExpr::Binario(OpBinario::Mul, _, _)));
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn los_operadores_son_asociativos_a_la_izquierda() {
    // (10 - 3) - 2
    match expr("10 - 3 - 2") {
        ClaseExpr::Binario(OpBinario::Resta, a, b) => {
            assert!(matches!(
                a.clase,
                ClaseExpr::Binario(OpBinario::Resta, _, _)
            ));
            assert_eq!(b.clase, ClaseExpr::Entero(2));
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn la_precedencia_sigue_el_orden_de_rust() {
    // a == (b | c)
    match expr("a == b | c") {
        ClaseExpr::Binario(OpBinario::Igual, _, d) => {
            assert!(matches!(d.clase, ClaseExpr::Binario(OpBinario::O, _, _)));
        }
        otro => panic!("{otro:?}"),
    }
    // (a & b) | (c & d)
    assert!(matches!(
        expr("a & b | c & d"),
        ClaseExpr::Binario(OpBinario::O, _, _)
    ));
    // a || (b && c)
    assert!(matches!(
        expr("a || b && c"),
        ClaseExpr::Binario(OpBinario::OLogico, _, _)
    ));
}

#[test]
fn los_parentesis_cambian_el_orden() {
    match expr("(1 + 2) * 3") {
        ClaseExpr::Binario(OpBinario::Mul, a, _) => {
            assert!(matches!(a.clase, ClaseExpr::Binario(OpBinario::Suma, _, _)));
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn operadores_de_saturacion_y_desplazamiento() {
    assert!(matches!(
        expr("a +| b"),
        ClaseExpr::Binario(OpBinario::SumaSat, _, _)
    ));
    assert!(matches!(
        expr("a << 2"),
        ClaseExpr::Binario(OpBinario::DespIzq, _, _)
    ));
}

#[test]
fn unarios_direccion_y_desreferencia() {
    assert!(matches!(expr("-x"), ClaseExpr::Unario(OpUnario::Negar, _)));
    assert!(matches!(
        expr("~x"),
        ClaseExpr::Unario(OpUnario::Complemento, _)
    ));
    assert!(matches!(expr("!x"), ClaseExpr::Unario(OpUnario::No, _)));
    assert!(matches!(expr("&x"), ClaseExpr::DireccionDe(_)));
    assert!(matches!(expr("*p"), ClaseExpr::Deref(_)));
}

#[test]
fn llamadas_indices_campos_y_conversiones() {
    assert!(matches!(expr("f(1, 2)"), ClaseExpr::Llamada(n, a) if n == "f" && a.len() == 2));
    assert!(matches!(expr("f()"), ClaseExpr::Llamada(_, a) if a.is_empty()));
    assert!(matches!(expr("t[3]"), ClaseExpr::Indice(_, _)));
    assert!(matches!(expr("p.x"), ClaseExpr::Campo(_, c) if c == "x"));
    assert!(matches!(expr("x como u16"), ClaseExpr::Cast(_, _)));
    // `como` liga mas que la suma: a + (b como u16)
    match expr("a + b como u16") {
        ClaseExpr::Binario(OpBinario::Suma, _, d) => {
            assert!(matches!(d.clase, ClaseExpr::Cast(_, _)))
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn constantes_y_variables_globales() {
    let p = programa("const MAX: u8 = 10; var cuenta: u16 @ 0x4000 = 5; var tabla: [u8; 4];");
    assert!(matches!(&p.elementos[0], Elemento::Const(c) if c.nombre == "MAX"));
    match &p.elementos[1] {
        Elemento::Global(g) => {
            assert!(g.direccion.is_some());
            assert!(g.inicial.is_some());
        }
        otro => panic!("{otro:?}"),
    }
    match &p.elementos[2] {
        Elemento::Global(g) => assert!(matches!(g.tipo.clase, ClaseTipo::Arreglo(_, _))),
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn estructuras_con_desplazamiento_fijado() {
    let p = programa("estructura Cabecera { tipo: u8, largo: u8 @ 2, datos: [u8; 4], }");
    match &p.elementos[0] {
        Elemento::Estructura(e) => {
            assert_eq!(e.campos.len(), 3);
            assert!(e.campos[1].desplazamiento.is_some());
            assert!(e.campos[0].desplazamiento.is_none());
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn funciones_con_parametros_retorno_y_atributo() {
    let p = programa("@interrupcion(1) funcion tecla(a: u8, b: *u8) -> u8 { devolver a; }");
    let f = funcion(&p);
    assert_eq!(f.nombre, "tecla");
    assert_eq!(f.parametros.len(), 2);
    assert!(matches!(f.parametros[1].tipo.clase, ClaseTipo::Puntero(_)));
    assert!(f.retorno.is_some());
    assert_eq!(f.atributos[0].nombre, "interrupcion");
    assert_eq!(f.atributos[0].argumentos.len(), 1);
}

#[test]
fn sentencias_de_control() {
    let p = programa(
        "funcion f() {
            var i: u8 = 0;
            mientras i < 10 { i += 1; si i == 5 { continuar; } sino si i == 8 { romper; } sino { i = i; } }
            bucle { romper; }
            segun i { 1 => { i = 2; } 2 => { i = 3; } otro => { i = 0; } }
            devolver;
        }",
    );
    let f = funcion(&p);
    assert_eq!(f.cuerpo.sentencias.len(), 5);
    assert!(matches!(
        f.cuerpo.sentencias[1].clase,
        ClaseSentencia::Mientras { .. }
    ));
    assert!(matches!(
        f.cuerpo.sentencias[2].clase,
        ClaseSentencia::Bucle(_)
    ));
    match &f.cuerpo.sentencias[3].clase {
        ClaseSentencia::Segun { brazos, otro, .. } => {
            assert_eq!(brazos.len(), 2);
            assert!(otro.is_some());
        }
        otro => panic!("{otro:?}"),
    }
    assert!(matches!(
        f.cuerpo.sentencias[4].clase,
        ClaseSentencia::Devolver(None)
    ));
}

#[test]
fn asignaciones_compuestas() {
    let p = programa("funcion f() { x += 1; x <<= 2; x = 3; t[1] = 4; p.a ^= 5; *q = 6; }");
    let f = funcion(&p);
    let ops: Vec<Option<OpBinario>> = f
        .cuerpo
        .sentencias
        .iter()
        .map(|s| match &s.clase {
            ClaseSentencia::Asignar { op, .. } => *op,
            otro => panic!("{otro:?}"),
        })
        .collect();
    assert_eq!(
        ops,
        vec![
            Some(OpBinario::Suma),
            Some(OpBinario::DespIzq),
            None,
            None,
            Some(OpBinario::Xor),
            None
        ]
    );
}

#[test]
fn ensamblador_en_linea_e_importaciones() {
    let p = programa("importar io { leer, escribir }; funcion f() { asm(\"NOP\"); }");
    assert!(
        matches!(&p.elementos[0], Elemento::Importar(i) if i.simbolos == vec!["leer", "escribir"])
    );
    match &funcion_en(&p, 1).cuerpo.sentencias[0].clase {
        ClaseSentencia::Asm(t) => assert_eq!(t, "NOP"),
        otro => panic!("{otro:?}"),
    }
}

fn funcion_en(p: &Programa, i: usize) -> &DefFuncion {
    match &p.elementos[i] {
        Elemento::Funcion(f) => f,
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn una_variable_sin_tipo_ni_valor_se_rechaza_con_sugerencia() {
    let e = error("funcion f() { var x; }");
    assert!(e.contains("ni tipo ni valor"), "{e}");
    assert!(e.contains("sugerencia"), "{e}");
}

#[test]
fn un_punto_y_coma_que_falta_se_senala_al_final_de_la_linea_anterior() {
    let e = error("funcion f() {\n    x = 3\n    y = 4;\n}");
    assert!(e.starts_with("t.sgt:2:"), "{e}");
    assert!(e.contains("falta `;`"), "{e}");
}

#[test]
fn una_llave_sin_cerrar_se_detecta_en_el_final_del_archivo() {
    let e = error("funcion f() {\n    x = 3;");
    assert!(e.contains("falta `}`"), "{e}");
}

#[test]
fn un_error_de_expresion_dice_que_encontro() {
    let e = error("funcion f() { x = ; }");
    assert!(
        e.contains("se esperaba una expresion y se encontro `;`"),
        "{e}"
    );
}

#[test]
fn algo_que_no_es_una_declaracion_en_el_primer_nivel_se_rechaza() {
    let e = error("x = 3;");
    assert!(e.contains("una declaracion"), "{e}");
}

#[test]
fn un_atributo_sin_funcion_se_rechaza() {
    let e = error("@interrupcion(1) var x: u8;");
    assert!(e.contains("`funcion`"), "{e}");
}

#[test]
fn el_lugar_de_cada_nodo_es_el_de_su_primer_lexema() {
    let p = programa("\n\nfuncion f() {\n  x = 1;\n}");
    let f = funcion(&p);
    assert_eq!((f.lugar.linea, f.lugar.columna), (3, 1));
    assert_eq!(f.cuerpo.sentencias[0].lugar.linea, 4);
}
