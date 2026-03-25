//! Ejecutar pruebas de proyecto (RF-TST-002 a RF-TST-010, RF-TST-013, RF-TST-014).

use std::path::PathBuf;

use lorenz_tst::ejecutor::{ejecutar, ejecutar_con_depurador, Resultado, Veredicto};
use lorenz_tst::formato::Proyecto;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn carpeta(nombre: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lorenz-tst-{}-{nombre}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Crea una carpeta con los programas dados y devuelve el proyecto descrito por `pruebas`.
fn proyecto_en(maquina: &str, nombre: &str, programas: &[(&str, &str)], pruebas: &str) -> Proyecto {
    let dir = carpeta(nombre);
    for (n, texto) in programas {
        std::fs::write(dir.join(n), texto).unwrap();
    }
    let m = raiz().join(maquina).join("maquina.toml");
    let texto = format!(
        "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\n{pruebas}",
        m.to_string_lossy()
    );
    Proyecto::desde_texto(&texto, "p.toml", &dir).unwrap_or_else(|e| panic!("{e}"))
}

fn proyecto(nombre: &str, programas: &[(&str, &str)], pruebas: &str) -> Proyecto {
    proyecto_en("ejemplos/ejemplo-8", nombre, programas, pruebas)
}

fn una(p: &Proyecto) -> Resultado {
    ejecutar(p, &p.pruebas[0])
}

const SUMA: &str = "LDI A, 200\nLDI B, 100\nADD A, B\nST [0x2000], A\nHLT\n";

#[test]
fn una_prueba_que_pasa() {
    let p = proyecto(
        "pasa",
        &[("suma.s", SUMA)],
        r#"
[[prueba]]
nombre = "suma"
programa = "suma.s"
[prueba.espera]
registros = { A = 44 }
banderas = { C = true, Z = false }
memoria = [ { direccion = 0x2000, palabras = [44] } ]
pc = 9
ciclos = { valor = 11, tolerancia = 0 }
"#,
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Pasa, "{:?}", r.fallos);
    assert_eq!(r.instrucciones, 5);
    assert!(r.fallos.is_empty());
}

#[test]
fn un_registro_distinto_dice_esperado_obtenido_y_donde() {
    let p = proyecto(
        "registro",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"suma\"\nprograma=\"suma.s\"\n[prueba.espera]\nregistros = { A = 45 }\n",
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Falla);
    assert_eq!(r.fallos.len(), 1);
    let f = &r.fallos[0];
    assert_eq!(f.que, "registro A");
    assert!(f.esperado.contains("45"), "{f:?}");
    assert!(f.obtenido.contains("44"), "{f:?}");
    let d = f.donde.as_ref().unwrap();
    assert_eq!(d.instruccion, "ADD A, B");
    assert_eq!(d.instante, 2);
    assert_eq!(d.fuente.as_ref().unwrap().1, 3);
}

#[test]
fn una_bandera_distinta() {
    let p = proyecto(
        "bandera",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nbanderas = { C = false }\n",
    );
    let r = una(&p);
    assert_eq!(r.fallos[0].que, "bandera C");
    assert_eq!(r.fallos[0].esperado, "0");
    assert_eq!(r.fallos[0].obtenido, "1");
    assert_eq!(r.fallos[0].donde.as_ref().unwrap().instruccion, "ADD A, B");
}

#[test]
fn memoria_distinta_senala_la_celda_y_la_instruccion_que_escribio() {
    let p = proyecto(
        "memoria",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nmemoria = [ { direccion = 0x2000, palabras = [44, 7] } ]\n",
    );
    let r = una(&p);
    let f = &r.fallos[0];
    assert!(
        f.que.contains("0x2000") && f.que.contains("difiere en 0x2001"),
        "{}",
        f.que
    );
    assert_eq!(f.esperado, "[2c 07]");
    assert_eq!(f.obtenido, "[2c 00]");
    // Nadie escribio 0x2001: se senala la ultima instruccion ejecutada.
    assert_eq!(f.donde.as_ref().unwrap().instruccion, "HLT");
}

#[test]
fn memoria_escrita_por_otra_instruccion() {
    let p = proyecto(
        "memoria2",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nmemoria = [ { direccion = 0x2000, palabras = [1] } ]\n",
    );
    let f = &una(&p).fallos[0];
    assert_eq!(f.donde.as_ref().unwrap().instruccion, "ST [0x2000], A");
}

#[test]
fn varios_fallos_se_informan_juntos() {
    let p = proyecto(
        "varios",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nregistros = { A = 1, B = 2 }\npc = 0\n",
    );
    assert_eq!(una(&p).fallos.len(), 3);
}

#[test]
fn el_limite_de_instrucciones_hace_fallar() {
    let p = proyecto(
        "limite",
        &[("bucle.s", "inicio: ADD A, B\n JZ inicio\n")],
        "[[prueba]]\nnombre=\"b\"\nprograma=\"bucle.s\"\nlimite = 100\n",
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Falla);
    assert_eq!(r.fallos[0].que, "detenida");
    assert!(
        r.fallos[0].obtenido.contains("100 instrucciones"),
        "{:?}",
        r.fallos
    );
    assert_eq!(r.instrucciones, 100);
}

#[test]
fn un_programa_que_no_debe_terminar_puede_esperarlo() {
    let p = proyecto(
        "sin-fin",
        &[("bucle.s", "inicio: ADD A, B\n JZ inicio\n")],
        "[[prueba]]\nnombre=\"b\"\nprograma=\"bucle.s\"\nlimite = 50\n[prueba.espera]\ndetenida = false\n",
    );
    assert_eq!(una(&p).veredicto, Veredicto::Pasa);
}

#[test]
fn estado_inicial_de_registros_banderas_y_memoria() {
    let p = proyecto(
        "inicial",
        &[("p.s", "LD A, [0x2000]\nADD A, B\nST [0x2001], A\nHLT\n")],
        r#"
[[prueba]]
nombre = "i"
programa = "p.s"
[prueba.inicial]
registros = { B = 3 }
banderas = { Z = true }
memoria = [ { direccion = 0x2000, palabras = [10] } ]
[prueba.espera]
registros = { A = 13 }
memoria = [ { direccion = 0x2001, palabras = [13] } ]
"#,
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Pasa, "{:?}", r.fallos);
}

#[test]
fn los_ciclos_admiten_tolerancia() {
    let mk = |n, valor, tol| {
        proyecto(
            n,
            &[("suma.s", SUMA)],
            &format!("[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nciclos = {{ valor = {valor}, tolerancia = {tol} }}\n"),
        )
    };
    // La suma consume 11 ciclos.
    assert!(una(&mk("c1", 13, 2)).paso());
    assert!(una(&mk("c2", 9, 2)).paso());
    let r = una(&mk("c3", 14, 2));
    assert!(!r.paso());
    assert_eq!(r.fallos[0].esperado, "14 (+/- 2)");
    assert_eq!(r.fallos[0].obtenido, "11");
    assert!(una(&mk("c4", 11, 0)).paso());
}

#[test]
fn programa_hexadecimal() {
    let p = proyecto(
        "hex",
        &[("p.hex", "20 05\n F0\n")],
        "[[prueba]]\nnombre=\"h\"\nprograma=\"p.hex\"\n[prueba.espera]\nregistros = { A = 5 }\n",
    );
    assert!(una(&p).paso());
}

#[test]
fn programa_que_no_ensambla_es_un_error_no_un_fallo() {
    let p = proyecto(
        "malo",
        &[("mal.s", "BAILAR A\n")],
        "[[prueba]]\nnombre=\"m\"\nprograma=\"mal.s\"\n",
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Error);
    assert!(r.motivo.unwrap().contains("mal.s"));
}

#[test]
fn programa_que_no_existe() {
    let p = proyecto(
        "falta",
        &[],
        "[[prueba]]\nnombre=\"m\"\nprograma=\"nada.s\"\n",
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Error);
    assert!(r.motivo.unwrap().contains("no se pudo leer"));
}

#[test]
fn un_fallo_de_ejecucion_se_informa_con_su_instruccion() {
    let p = proyecto(
        "acceso",
        &[("p.s", "LDI A, 5\nST [0x0010], A\nHLT\n")],
        "[[prueba]]\nnombre=\"x\"\nprograma=\"p.s\"\n",
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Falla);
    assert_eq!(r.fallos[0].que, "ejecucion");
    assert!(r.fallos[0].obtenido.contains("solo lectura"));
    assert_eq!(
        r.fallos[0].donde.as_ref().unwrap().instruccion,
        "ST [0x0010], A"
    );
}

#[test]
fn entradas_simuladas_y_salida_de_la_uart() {
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "eco",
        &[("eco.s", "IN A\nOUT A\nIN A\nOUT A\nHLT\n")],
        r#"
[[prueba]]
nombre = "eco"
programa = "eco.s"
[[prueba.entrada]]
dispositivo = "serie"
texto = "ok"
[prueba.espera]
registros = { A = 107 }
salida = [ { dispositivo = "serie", texto = "ok" } ]
"#,
    );
    let r = una(&p);
    assert_eq!(r.veredicto, Veredicto::Pasa, "{:?}", r.fallos);
}

#[test]
fn salida_distinta_muestra_ambos_textos() {
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "eco2",
        &[("eco.s", "IN A\nOUT A\nHLT\n")],
        r#"
[[prueba]]
nombre = "eco"
programa = "eco.s"
[[prueba.entrada]]
dispositivo = "serie"
texto = "Z"
[prueba.espera]
salida = [ { dispositivo = "serie", texto = "Y" } ]
"#,
    );
    let f = &una(&p).fallos[0];
    assert_eq!(f.que, "salida de serie");
    assert_eq!(f.esperado, "\"Y\"");
    assert_eq!(f.obtenido, "\"Z\"");
}

#[test]
fn salida_con_codigos() {
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "eco3",
        &[("p.s", "LDI A, 200\nOUT A\nHLT\n")],
        "[[prueba]]\nnombre=\"c\"\nprograma=\"p.s\"\n[prueba.espera]\nsalida = [ { dispositivo = \"serie\", codigos = [200] } ]\n",
    );
    assert!(una(&p).paso());
}

#[test]
fn puertos_de_salida_en_bruto() {
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "puerto",
        &[("p.s", "LDI A, 7\nOUT A\nOUT A\nHLT\n")],
        "[[prueba]]\nnombre=\"p\"\nprograma=\"p.s\"\n[prueba.espera]\npuertos = [ { puerto = 0x10, valores = [7, 7] } ]\n",
    );
    assert!(una(&p).paso());
}

#[test]
fn entrada_en_un_instante_dado() {
    // La tecla llega antes de la instruccion 2: la primera lectura no ve nada.
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "tarde",
        &[("p.s", "IN A\nIN B\nHLT\n")],
        r#"
[[prueba]]
nombre = "tarde"
programa = "p.s"
[[prueba.entrada]]
dispositivo = "serie"
texto = "Q"
en_instruccion = 1
[prueba.espera]
registros = { A = 0, B = 81 }
"#,
    );
    assert!(una(&p).paso(), "{:?}", una(&p).fallos);
}

#[test]
fn interrupciones_en_un_instante_dado() {
    // Vector 1 apunta a 0xEFF2; ahi dejamos un manejador que cuenta en C y vuelve.
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "irq",
        &[(
            "p.s",
            "        .org 0x0000\n        EI\n        LDI B, 1\n        LDI B, 2\n        LDI B, 3\n        HLT\n        .org 0xEFF2\n        .dw 0x0100\n        .org 0x0100\n        LDI C, 99\n        RETI\n",
        )],
        r#"
[[prueba]]
nombre = "irq"
programa = "p.s"
[[prueba.interrupcion]]
linea = 1
en_instruccion = 2
[prueba.espera]
registros = { B = 3, C = 99 }
"#,
    );
    let r = una(&p);
    assert!(r.paso(), "{:?} {:?}", r.fallos, r.motivo);
}

#[test]
fn el_depurador_queda_en_el_estado_en_que_fallo() {
    let p = proyecto(
        "depurar",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nregistros = { A = 1 }\n",
    );
    let (r, d) = ejecutar_con_depurador(&p, &p.pruebas[0]);
    assert_eq!(r.veredicto, Veredicto::Falla);
    let mut d = d.expect("debe haber depurador");
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 44);
    assert!(d.emulador().estado().detenida());
    // Y se puede retroceder hasta antes de la suma.
    d.ir_a(2).unwrap();
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 200);
}

#[test]
fn si_pasa_no_hay_depurador() {
    let p = proyecto(
        "nodep",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n",
    );
    assert!(ejecutar_con_depurador(&p, &p.pruebas[0]).1.is_none());
}

#[test]
fn la_ejecucion_es_repetible() {
    let p = proyecto(
        "repite",
        &[("suma.s", SUMA)],
        "[[prueba]]\nnombre=\"s\"\nprograma=\"suma.s\"\n[prueba.espera]\nregistros = { A = 1 }\n",
    );
    let a = una(&p);
    let b = una(&p);
    assert_eq!(a.fallos, b.fallos);
    assert_eq!(a.instrucciones, b.instrucciones);
}

// --- pulsos de reloj (RF-TST-008) ---

fn programa_con_manejador() -> String {
    let nops = "        NOP\n".repeat(80);
    format!(
        "        .org 0x0000\n        EI\n{nops}        HLT\n        .org 0xEFF2\n        .dw 0x0100\n        .org 0x0100\n        LDI D, 1\n        ADD C, D\n        RETI\n"
    )
}

#[test]
fn un_pulso_de_reloj_levanta_la_linea_en_cada_periodo() {
    let programa = programa_con_manejador();
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "pulsos",
        &[("p.s", programa.as_str())],
        r#"
[[prueba]]
nombre = "tres pulsos"
programa = "p.s"
limite = 500
[[prueba.pulso]]
linea = 1
desde_ciclo = 5
periodo = 10
veces = 3
[prueba.espera]
registros = { C = 3 }
"#,
    );
    let r = una(&p);
    assert!(r.paso(), "{:?} {:?}", r.fallos, r.motivo);
}

#[test]
fn sin_pulsos_el_manejador_no_corre() {
    let programa = programa_con_manejador();
    let p = proyecto_en(
        "ejemplos/ejemplo-8-serie",
        "sinpulsos",
        &[("p.s", programa.as_str())],
        "[[prueba]]\nnombre = \"nada\"\nprograma = \"p.s\"\nlimite = 500\n[prueba.espera]\nregistros = { C = 0 }\n",
    );
    assert!(una(&p).paso());
}

/// Carga un proyecto cuyo texto de pruebas es `pruebas` y devuelve el mensaje de error.
fn error_de_carga(pruebas: &str) -> String {
    let dir = carpeta("pulso-error");
    std::fs::write(dir.join("p.s"), "HLT\n").unwrap();
    let m = raiz().join("ejemplos/ejemplo-8-serie/maquina.toml");
    let texto = format!(
        "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\n{pruebas}",
        m.to_string_lossy()
    );
    match Proyecto::desde_texto(&texto, "p.toml", &dir) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("se esperaba un error de carga"),
    }
}

#[test]
fn un_pulso_sin_periodo_valido_se_rechaza() {
    for malo in ["periodo = 0", "periodo = 5\nveces = 0"] {
        let e = error_de_carga(&format!(
            "[[prueba]]\nnombre = \"x\"\nprograma = \"p.s\"\n[[prueba.pulso]]\nlinea = 1\n{malo}\n"
        ));
        assert!(e.contains("pulso"), "{malo}: {e}");
    }
}

#[test]
fn un_pulso_en_una_linea_sin_vector_se_rechaza() {
    let e = error_de_carga(
        "[[prueba]]\nnombre = \"x\"\nprograma = \"p.s\"\n[[prueba.pulso]]\nlinea = 7\nperiodo = 5\n",
    );
    assert!(e.contains("no tiene vector"), "{e}");
}
