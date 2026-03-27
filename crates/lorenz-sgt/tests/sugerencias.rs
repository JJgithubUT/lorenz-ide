//! Los errores con una causa reconocible proponen la correccion (RF-SGT-037).
mod comun;
use comun::*;

fn error(fuente: &str) -> String {
    compilar_en("risc-16", fuente).unwrap_err()
}

fn sugiere(fuente: &str, texto: &str) {
    let e = error(fuente);
    let sug = e
        .lines()
        .skip_while(|l| !l.contains("sugerencia:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        sug.contains(texto),
        "se esperaba una sugerencia con `{texto}`:\n{e}"
    );
}

#[test]
fn nombre_escrito_casi_igual() {
    sugiere(
        "var contador: u16; funcion principal() { contado = 1; }",
        "`contador`",
    );
}

#[test]
fn tipo_escrito_casi_igual() {
    sugiere("var x: u61; funcion principal() { }", "u16");
}

#[test]
fn signo_distinto_sugiere_como() {
    sugiere(
        "var a: u16; var b: i16; funcion principal() { a = b; }",
        "como u16",
    );
}

#[test]
fn argumento_de_otro_tipo_sugiere_como() {
    sugiere(
        "funcion f(x: u16) { } funcion principal() { var s: i16 = 1; f(s); }",
        "como u16",
    );
}

#[test]
fn devolver_otro_tipo_sugiere_como() {
    sugiere(
        "funcion f(x: i16) -> u16 { devolver x; } funcion principal() { }",
        "como u16",
    );
}

#[test]
fn entero_como_condicion_sugiere_comparar() {
    sugiere("funcion principal() { var x: u16 = 1; si x { } }", "!= 0");
}

#[test]
fn entero_donde_va_un_booleano() {
    sugiere(
        "var b: bool; funcion principal() { var x: u16 = 1; b = x; }",
        "!= 0",
    );
}

#[test]
fn indice_de_otro_tipo() {
    sugiere(
        "var v: [u16; 4]; funcion principal() { var i: bool = verdadero; v[i] = 1; }",
        "como u16",
    );
}

#[test]
fn faltan_argumentos_y_se_muestra_la_firma() {
    sugiere(
        "funcion f(a: u16, b: i16) { } funcion principal() { f(1); }",
        "f(u16, i16)",
    );
}

#[test]
fn romper_fuera_de_un_bucle() {
    sugiere("funcion principal() { romper; }", "mientras");
}

#[test]
fn devolver_sin_valor_en_funcion_con_retorno() {
    sugiere(
        "funcion f() -> u16 { devolver; } funcion principal() { }",
        "devolver valor;",
    );
}

#[test]
fn devolver_valor_en_funcion_sin_retorno() {
    sugiere(
        "funcion f() { devolver 1; } funcion principal() { }",
        "-> tipo",
    );
}

#[test]
fn dividir_entre_cero() {
    sugiere("var r: u16; funcion principal() { r = 5 / 0; }", "divisor");
}

#[test]
fn recursion() {
    sugiere("funcion f() { f(); } funcion principal() { f(); }", "bucle");
}

#[test]
fn falta_principal() {
    sugiere("var x: u16;", "funcion principal()");
}

#[test]
fn registro_inexistente_lista_los_que_hay() {
    let e = error("funcion principal() { reg.Q9 = 1; }");
    assert!(
        e.contains("no tiene el registro `Q9`") && e.contains("R1"),
        "{e}"
    );
}

#[test]
fn campo_inexistente_lista_los_que_hay() {
    let e = error("estructura P { x: u16, y: u16 } var p: P; funcion principal() { p.z = 1; }");
    assert!(e.contains("tiene: x, y"), "{e}");
}
