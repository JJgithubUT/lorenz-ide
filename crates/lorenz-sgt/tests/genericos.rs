//! Genericos sencillos sobre tipos enteros (RF-SGT-021).
mod comun;
use comun::*;

const MAQ: &str = "sgt-8";

fn error(fuente: &str) -> String {
    compilar_en(MAQ, fuente).err().expect("debia fallar")
}

#[test]
fn una_funcion_generica_se_instancia_con_u8() {
    let (g, _) = ejecutado(
        MAQ,
        "var r: u8;
         funcion maximo<T>(a: T, b: T) -> T { si a > b { devolver a; } devolver b; }
         funcion principal() { r = maximo<u8>(7, 200); }",
    );
    assert_eq!(g["r"], 200);
}

#[test]
fn el_mismo_generico_con_dos_tipos_da_dos_copias_independientes() {
    let fuente = "var a: u8; var b: u16;
         funcion doble<T>(x: T) -> T { devolver x + x; }
         funcion principal() { a = doble<u8>(100); b = doble<u16>(40000); }";
    let (g, _) = ejecutado(MAQ, fuente);
    assert_eq!(g["a"], 200);
    assert_eq!(g["b"], 80000 & 0xFFFF);
    let c = compilar_en(MAQ, fuente).unwrap();
    let nombres: Vec<&str> = c
        .modulo
        .funciones
        .iter()
        .map(|f| f.nombre.as_str())
        .collect();
    assert!(
        nombres.contains(&"doble__u8") && nombres.contains(&"doble__u16"),
        "{nombres:?}"
    );
}

#[test]
fn el_resultado_coincide_con_el_oraculo() {
    igual_al_oraculo(
        MAQ,
        "var a: u8; var b: i8; var c: u16;
         funcion menor<T>(x: T, y: T) -> T { si x < y { devolver x; } devolver y; }
         funcion principal() { a = menor<u8>(9, 3); b = menor<i8>(-5, 4); c = menor<u16>(300, 1000); }",
    );
}

#[test]
fn dos_llamadas_iguales_comparten_la_copia() {
    let c = compilar_en(
        MAQ,
        "var r: u8;
         funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { r = f<u8>(1) + f<u8>(2); }",
    )
    .unwrap();
    let copias = c
        .modulo
        .funciones
        .iter()
        .filter(|f| f.nombre.starts_with("f__"))
        .count();
    assert_eq!(copias, 1);
}

#[test]
fn un_generico_puede_llamar_a_otro() {
    let (g, _) = ejecutado(
        MAQ,
        "var r: u8;
         funcion sumar<T>(a: T, b: T) -> T { devolver a + b; }
         funcion triple<T>(a: T) -> T { devolver sumar<T>(a, sumar<T>(a, a)); }
         funcion principal() { r = triple<u8>(5); }",
    );
    assert_eq!(g["r"], 15);
}

#[test]
fn la_recursion_se_rechaza_como_en_cualquier_funcion_y_nombra_la_copia() {
    let e = error(
        "funcion suma_hasta<T>(n: T) -> T { si n == 0 { devolver 0; } devolver n + suma_hasta<T>(n - 1); }
         funcion principal() { var r: u8 = suma_hasta<u8>(4); }",
    );
    assert!(
        e.contains("suma_hasta__u8") && e.contains("se llama a si misma"),
        "{e}"
    );
}

#[test]
fn el_parametro_de_tipo_sirve_en_variables_conversiones_y_punteros() {
    // En `sgt-8` un puntero ocupa dos palabras; en `sgt-16`, una.
    let (g, _) = ejecutado(
        "sgt-16",
        "var r: u16; var s: u16;
         funcion f<T>(p: *T, x: T) -> T { var y: T = x; *p = y + 1; devolver (x como T) + 2; }
         funcion principal() { var v: u16 = 0; r = f<u16>(&v, 10); s = v; }",
    );
    assert_eq!((g["r"], g["s"]), (12, 11));
}

#[test]
fn dos_parametros_de_tipo() {
    let (g, _) = ejecutado(
        MAQ,
        "var r: u16;
         funcion ensanchar<A, B>(x: A) -> B { devolver x como B; }
         funcion principal() { r = ensanchar<u8, u16>(200); }",
    );
    assert_eq!(g["r"], 200);
}

#[test]
fn una_comparacion_normal_no_se_confunde_con_una_llamada_generica() {
    let (g, _) = ejecutado(
        MAQ,
        "var r: u8;
         funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { var a: u8 = 3; var b: u8 = 9; si a < b && b > a { r = f<u8>(4); } }",
    );
    assert_eq!(g["r"], 4);
}

#[test]
fn un_generico_sin_usar_no_deja_codigo() {
    let c = compilar_en(
        MAQ,
        "funcion f<T>(x: T) -> T { devolver x; } funcion principal() { }",
    )
    .unwrap();
    assert!(c
        .modulo
        .funciones
        .iter()
        .all(|f| !f.nombre.starts_with("f")));
}

#[test]
fn llamar_a_un_generico_sin_tipos_dice_como_escribirlo() {
    let e = error(
        "funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { var a: u8 = f(1); }",
    );
    assert!(e.contains("necesita que se diga con que tipos"), "{e}");
    assert!(e.contains("f<u8>(...)"), "{e}");
}

#[test]
fn un_tipo_que_no_es_entero_se_rechaza() {
    let e = error(
        "funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { var a: bool = f<bool>(true); }",
    );
    assert!(e.contains("solo se instancia con tipos enteros"), "{e}");
}

#[test]
fn el_numero_de_tipos_debe_cuadrar() {
    let e = error(
        "funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { var a: u8 = f<u8, u16>(1); }",
    );
    assert!(e.contains("1 parametro de tipo y se le dan 2"), "{e}");
}

#[test]
fn llamar_con_tipos_a_una_funcion_corriente_o_inexistente_es_un_error() {
    let e = error(
        "funcion g(x: u8) -> u8 { devolver x; }
         funcion principal() { var a: u8 = g<u8>(1); }",
    );
    assert!(e.contains("no es una funcion generica"), "{e}");
    let e = error("funcion principal() { var a: u8 = h<u8>(1); }");
    assert!(e.contains("no hay una funcion generica llamada `h`"), "{e}");
}

#[test]
fn un_ancho_que_la_maquina_no_tiene_lo_dice_el_analisis() {
    let e = error(
        "funcion f<T>(x: T) -> T { devolver x; }
         funcion principal() { var a: u8 = f<u7>(1); }",
    );
    assert!(e.contains("u7"), "{e}");
}

#[test]
fn una_funcion_generica_no_puede_llevar_atributos() {
    let e = error(
        "@interrupcion(1) funcion f<T>() { }
         funcion principal() { }",
    );
    assert!(e.contains("no puede llevar atributos"), "{e}");
}

#[test]
fn el_error_dentro_de_una_copia_se_senala_en_el_cuerpo_del_generico() {
    let e = error(
        "funcion f<T>(x: T) -> T { devolver y; }
         funcion principal() { var a: u8 = f<u8>(1); }",
    );
    assert!(e.contains("y"), "{e}");
    assert!(e.contains("t.sgt:1"), "{e}");
}
