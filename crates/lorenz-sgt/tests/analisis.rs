//! El analisis semantico de SGT contra una maquina destino.

use lorenz_arch::carga::cargar_archivo;
use lorenz_sgt::analisis::{analizar, Opciones};
use lorenz_sgt::destino::Destino;
use lorenz_sgt::hir::Modulo;
use lorenz_sgt::sintaxis::analizar_programa;

fn destino(nombre: &str) -> Destino {
    let ruta = format!(
        "{}/../../ejemplos/{nombre}/maquina.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    Destino::de(&cargar_archivo(ruta).unwrap())
}

fn compilar_en(maq: &str, fuente: &str) -> Result<Modulo, Vec<String>> {
    let p = analizar_programa("t.sgt", fuente).unwrap_or_else(|e| panic!("{e}"));
    analizar("t.sgt", &p, &destino(maq), Opciones::default())
        .map_err(|v| v.iter().map(|d| d.to_string()).collect())
}

fn ok(fuente: &str) -> Modulo {
    compilar_en("risc-16", fuente).unwrap_or_else(|e| panic!("{}", e.join("\n")))
}

fn error(fuente: &str) -> String {
    compilar_en("risc-16", fuente).unwrap_err().join("\n")
}

fn error_en(maq: &str, fuente: &str) -> String {
    compilar_en(maq, fuente).unwrap_err().join("\n")
}

#[test]
fn programa_minimo_valido() {
    let m = ok("funcion principal() { }");
    assert_eq!(m.funciones.len(), 1);
}

#[test]
fn falta_principal() {
    let e = error("funcion otra() { }");
    assert!(e.contains("falta la funcion `principal`"), "{e}");
}

#[test]
fn principal_no_lleva_parametros() {
    let e = error("funcion principal(x: u16) { }");
    assert!(e.contains("no puede tener parametros"), "{e}");
}

#[test]
fn anchos_dependen_de_la_maquina() {
    // risc-16: palabra de 16 bits -> u16, u32, u64.
    ok("var x: u32; funcion principal() { }");
    let e = error("var x: u8; funcion principal() { }");
    assert!(
        e.contains("no existe en `risc-16`") && e.contains("16 bits"),
        "{e}"
    );
    // ejemplo-8-serie: palabra de 8 bits -> u8 existe y u12 no.
    compilar_en("ejemplo-8-serie", "var x: u8; funcion principal() { }").unwrap();
    let e = error_en("ejemplo-8-serie", "var x: u12; funcion principal() { }");
    assert!(e.contains("no existe"), "{e}");
}

#[test]
fn tipo_desconocido_sugiere() {
    let e = error("var x: u61; funcion principal() { }");
    assert!(e.contains("no existe"), "{e}");
    let e = error("estructura Punto { x: u16 } var p: Pnto; funcion principal() { }");
    assert!(e.contains("quiza querias decir `Punto`"), "{e}");
}

#[test]
fn literal_toma_el_tipo_esperado() {
    ok("funcion principal() { var a: u32 = 70000; var b: i16 = -5; }");
}

#[test]
fn literal_fuera_de_rango() {
    let e = error("funcion principal() { var a: u16 = 70000; }");
    assert!(e.contains("70000 no cabe en u16"), "{e}");
    let e = error("funcion principal() { var a: i16 = 40000; }");
    assert!(e.contains("no cabe en i16"), "{e}");
}

#[test]
fn minimo_con_signo_cabe() {
    ok("funcion principal() { var a: i16 = -32768; }");
}

#[test]
fn constantes_se_pliegan_con_la_aritmetica_de_sgt() {
    let m = ok("const N: u16 = 65535 + 2; const M: u16 = 3 +| 65535; funcion principal() { var a: u16 = N; var b: u16 = M; }");
    let f = &m.funciones[0];
    assert_eq!(f.locales.len(), 2);
}

#[test]
fn division_por_cero_constante() {
    let e = error("funcion principal() { var a: u16 = 1 / 0; }");
    assert!(e.contains("division entre cero"), "{e}");
}

#[test]
fn tipos_distintos_no_se_mezclan() {
    let e = error("funcion principal() { var a: u16 = 1; var b: u32 = 2; var c: u32 = a + b; }");
    assert!(
        e.contains("mezcla u16 con u32") && e.contains("como"),
        "{e}"
    );
}

#[test]
fn conversion_explicita() {
    ok("funcion principal() { var a: u16 = 1; var b: u32 = a como u32; var c: u16 = b como u16; }");
}

#[test]
fn nombre_desconocido_sugiere() {
    let e = error("funcion principal() { var contador: u16 = 0; contdor = 1; }");
    assert!(
        e.contains("no existe `contdor`") && e.contains("`contador`"),
        "{e}"
    );
}

#[test]
fn condicion_debe_ser_bool() {
    let e = error("funcion principal() { var a: u16 = 1; si a { } }");
    assert!(e.contains("debe ser bool"), "{e}");
    ok("funcion principal() { var a: u16 = 1; si a != 0 { } }");
}

#[test]
fn romper_fuera_de_bucle() {
    let e = error("funcion principal() { romper; }");
    assert!(e.contains("solo vale dentro de un bucle"), "{e}");
    ok("funcion principal() { mientras verdadero { romper; } }");
}

#[test]
fn falta_devolver() {
    let e = error(
        "funcion f(x: u16) -> u16 { si x == 0 { devolver 1; } } funcion principal() { f(1); }",
    );
    assert!(e.contains("puede terminar sin devolver"), "{e}");
    ok("funcion f(x: u16) -> u16 { si x == 0 { devolver 1; } sino { devolver 2; } } funcion principal() { f(1); }");
}

#[test]
fn llamada_con_argumentos_incorrectos() {
    let e = error("funcion f(x: u16) -> u16 { devolver x; } funcion principal() { f(1, 2); }");
    assert!(e.contains("recibe 1 argumentos"), "{e}");
    let e = error(
        "funcion f(x: u16) -> u16 { devolver x; } funcion principal() { var a: u32 = 1; f(a); }",
    );
    assert!(e.contains("espera u16"), "{e}");
}

#[test]
fn recursion_se_rechaza() {
    let e = error("funcion f() { f(); } funcion principal() { f(); }");
    assert!(e.contains("se llama a si misma"), "{e}");
    let e = error("funcion a() { b(); } funcion b() { a(); } funcion principal() { a(); }");
    assert!(e.contains("se llama a si misma"), "{e}");
}

#[test]
fn estructuras_y_campos() {
    let m = ok("estructura P { x: u16, y: u16 } var p: P; funcion principal() { p.y = 3; }");
    assert_eq!(m.estructuras[0].tamano, 2);
    assert_eq!(m.estructuras[0].campos[1].desplazamiento, 1);
    let e = error("estructura P { x: u16 } var p: P; funcion principal() { p.z = 3; }");
    assert!(
        e.contains("no tiene el campo `z`") && e.contains("tiene: x"),
        "{e}"
    );
}

#[test]
fn campos_solapados() {
    let e = error("estructura P { a: u32 @ 0, b: u16 @ 1 } funcion principal() { }");
    assert!(e.contains("se solapan"), "{e}");
}

#[test]
fn arreglos_y_limites() {
    ok("var t: [u16; 4]; funcion principal() { t[3] = 1; }");
    let e = error("var t: [u16; 4]; funcion principal() { t[4] = 1; }");
    assert!(e.contains("se sale de un arreglo de 4"), "{e}");
}

#[test]
fn ubicacion_en_regiones() {
    ok("var x: u16 @ 0x4000; funcion principal() { x = 1; }");
    let e = error("var x: u16 @ 0x4000 = 5; var y: u16 @ 0x0010 = 1; funcion principal() { }");
    assert!(e.contains("solo lectura"), "{e}");
}

#[test]
fn ubicacion_fuera_de_toda_region() {
    let e = error_en(
        "ejemplo-8-serie",
        "var x: u8 @ 0xF800; funcion principal() { }",
    );
    assert!(e.contains("no pertenece a ninguna region"), "{e}");
}

#[test]
fn escritura_en_rom() {
    let e = error("var x: u16 @ 0x0010; funcion principal() { x = 1; }");
    assert!(e.contains("solo lectura") && e.contains("rom"), "{e}");
    let e = error("funcion principal() { *(16 como *u16) = 1; }");
    assert!(e.contains("solo lectura") && e.contains("0x10"), "{e}");
}

#[test]
fn registros_de_la_maquina() {
    ok("funcion principal() { reg.R1 = 5; var a: u16 = reg.R1; }");
    let e = error("funcion principal() { reg.Q9 = 5; }");
    assert!(
        e.contains("no tiene el registro `Q9`") && e.contains("R1"),
        "{e}"
    );
}

#[test]
fn banderas_de_la_maquina() {
    compilar_en(
        "ejemplo-8-serie",
        "funcion principal() { si bandera.Z { } }",
    )
    .unwrap();
    let e = error_en(
        "ejemplo-8-serie",
        "funcion principal() { si bandera.X { } }",
    );
    assert!(e.contains("no tiene la bandera `X`"), "{e}");
}

#[test]
fn puertos_declarados() {
    compilar_en(
        "ejemplo-8-serie",
        "funcion principal() { var c: u8 = entrada(0x10); salida(0x10, c); }",
    )
    .unwrap();
    let e = error_en(
        "ejemplo-8-serie",
        "funcion principal() { salida(0x55, 1); }",
    );
    assert!(
        e.contains("el puerto 0x55 no esta declarado") && e.contains("0x10"),
        "{e}"
    );
}

#[test]
fn interrupciones_con_vector() {
    compilar_en(
        "ejemplo-8-serie",
        "@interrupcion(1) funcion tecla() { } funcion principal() { }",
    )
    .unwrap();
    let e = error_en(
        "ejemplo-8-serie",
        "@interrupcion(5) funcion tecla() { } funcion principal() { }",
    );
    assert!(e.contains("no tiene vector declarado"), "{e}");
}

#[test]
fn punteros_y_direcciones() {
    ok("var x: u16; funcion principal() { var p: *u16 = &x; *p = 3; var q: *u16 = p + 1; var y: u16 = q[0]; }");
    let e = error("funcion principal() { var a: u16 = 1; *a = 2; }");
    assert!(e.contains("necesita un puntero"), "{e}");
}

#[test]
fn segun_con_brazos_repetidos() {
    let e = error("funcion principal() { var a: u16 = 1; segun a { 1 => { } 1 => { } } }");
    assert!(e.contains("ya tiene un brazo"), "{e}");
}

#[test]
fn varios_errores_se_dan_juntos_y_ordenados() {
    let e = error("funcion principal() { x = 1; y = 2; }");
    let lineas: Vec<&str> = e.lines().filter(|l| l.starts_with("t.sgt:")).collect();
    assert_eq!(lineas.len(), 2, "{e}");
}

#[test]
fn sentencia_sin_efecto() {
    let e = error("funcion principal() { var a: u16 = 1; a + 1; }");
    assert!(e.contains("no hace nada"), "{e}");
}
