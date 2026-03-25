//! Puertos, registros, ensamblador en linea e interrupciones en sgt-16.

mod comun;
use comun::*;

#[test]
fn escribe_por_la_uart() {
    let c = correr(
        "sgt-16",
        "funcion principal() { salida(0x10, 72); salida(0x10, 105); salida(0x10, 33); }",
        b"",
    );
    assert_eq!(c.texto, "Hi!");
}

#[test]
fn lee_de_la_uart_y_repite() {
    let c = correr(
        "sgt-16",
        "funcion principal() { var i: u16 = 0; mientras i < 3 { var c: u16 = entrada(0x10); salida(0x10, c + 1); i += 1; } }",
        b"ABC",
    );
    assert_eq!(c.texto, "BCD");
}

#[test]
fn puerto_no_declarado_se_rechaza_en_terminos_de_la_maquina() {
    let e = compilar_en("sgt-16", "funcion principal() { salida(0x77, 1); }").unwrap_err();
    assert!(e.contains("0x77") && e.contains("0x10"), "{e}");
}

#[test]
fn maquina_sin_instruccion_de_salida() {
    // risc-16 no tiene puertos: el analisis ya lo dice.
    let e = compilar_en("risc-16", "funcion principal() { salida(0x10, 1); }").unwrap_err();
    assert!(e.contains("no declara ningun puerto"), "{e}");
}

#[test]
fn ensamblador_en_linea() {
    let c = correr(
        "sgt-16",
        "var x: u16; var y: u16 = 7;
         funcion principal() { asm(\"LDI R2, {y}\\nLW R3, [R2+0]\\nADDI R3, R3, 100\\nLDI R2, {x}\\nSW R3, [R2+0]\"); y = x + 1; }",
        b"",
    );
    assert_eq!(c.globales["x"], 107);
    assert_eq!(c.globales["y"], 108);
}

#[test]
fn registros_propios_del_compilador_no_se_tocan() {
    let e = compilar_en("sgt-16", "funcion principal() { reg.R3 = 1; }").unwrap_err();
    assert!(e.contains("lo usa el compilador"), "{e}");
}

#[test]
fn registro_inaccesible_se_explica() {
    let e = compilar_en("risc-16", "funcion principal() { reg.PC = 1; }").unwrap_err();
    assert!(e.contains("`PC`") && e.contains("risc-16"), "{e}");
}

#[test]
fn fijar_el_puntero_de_pila() {
    let c = correr(
        "sgt-16",
        "var s: u16; var t: u16; funcion principal() { reg.SP = 0x5000; s = reg.SP; t = 3; }",
        b"",
    );
    assert_eq!(c.globales["s"], 0x5000);
}

#[test]
fn registro_de_pila() {
    let c = correr(
        "sgt-16",
        "var s: u16; funcion principal() { s = reg.SP; }",
        b"",
    );
    assert_eq!(c.globales["s"], 0xDFFF);
}

#[test]
fn manejador_de_interrupcion() {
    let c = correr(
        "sgt-16",
        "var ticks: u16; var suma: u16;
         @interrupcion(0) funcion reloj() { ticks += 1; }
         funcion principal() { mientras ticks < 3 { suma += 1; } }",
        b"",
    );
    assert_eq!(c.globales["ticks"], 3);
    assert!(c.globales["suma"] > 100);
}

#[test]
fn manejador_y_programa_comparten_funciones_sin_pisarse() {
    // `x * 3` usa la biblioteca solo si la maquina no multiplica; aqui se fuerza con una
    // funcion del usuario llamada desde las dos raices.
    let c = correr(
        "sgt-16",
        "var ticks: u16; var lento: u16;
         funcion triple(a: u16) -> u16 { var t: u16 = a + a; devolver t + a; }
         @interrupcion(0) funcion reloj() { ticks = triple(ticks) + 1; }
         funcion principal() { var i: u16 = 0; mientras ticks < 20 { lento = triple(i); i += 1; } }",
        b"",
    );
    assert!(c.globales["ticks"] >= 20);
}

#[test]
fn linea_sin_vector_se_rechaza() {
    let e = compilar_en(
        "sgt-16",
        "@interrupcion(5) funcion f() { } funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("no tiene vector declarado"), "{e}");
}
