//! Pruebas de las ordenes de la linea de comandos, ejecutadas en el mismo proceso.

use std::path::PathBuf;

use lorenz_cli::{
    ejecutar_cli, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK,
};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maquina() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn programa_de_ejemplo(nombre: &str) -> String {
    raiz()
        .join("ejemplos/ejemplo-8/programas")
        .join(nombre)
        .to_string_lossy()
        .into_owned()
}

/// Escribe un archivo temporal unico para esta prueba y devuelve su ruta.
fn temporal(nombre: &str, contenido: &str) -> String {
    let dir = std::env::temp_dir().join(format!("lorenz-cli-pruebas-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join(nombre);
    std::fs::write(&ruta, contenido).unwrap();
    ruta.to_string_lossy().into_owned()
}

struct Resultado {
    codigo: i32,
    salida: String,
    errores: String,
}

fn correr(args: &[&str]) -> Resultado {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut salida = Vec::new();
    let mut errores = Vec::new();
    let codigo = ejecutar_cli(&args, &mut salida, &mut errores);
    Resultado {
        codigo,
        salida: String::from_utf8(salida).unwrap(),
        errores: String::from_utf8(errores).unwrap(),
    }
}

#[test]
fn ayuda_lista_las_ordenes_y_los_codigos_de_salida() {
    let r = correr(&["ayuda"]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("lorenz validar"));
    assert!(r.salida.contains("lorenz ejecutar"));
    assert!(r.salida.contains("Codigos de salida"));
}

#[test]
fn version_imprime_el_numero_del_paquete() {
    let r = correr(&["version"]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert_eq!(r.salida, format!("lorenz {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn sin_orden_devuelve_error_de_uso() {
    let r = correr(&[]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("falta la orden"));
}

#[test]
fn validar_acepta_el_ejemplo() {
    let r = correr(&["validar", &maquina()]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("valida"));
    assert!(r.salida.contains("ejemplo-8"));
}

#[test]
fn validar_un_archivo_inexistente_es_error_de_uso() {
    let r = correr(&["validar", "/no/existe.toml"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("/no/existe.toml"));
}

#[test]
fn validar_una_arquitectura_rota_lista_los_fallos() {
    let ruta = temporal(
        "rota.toml",
        "[maquina]\nnombre = \"x\"\nformato = \"9.9\"\n",
    );
    let r = correr(&["validar", &ruta]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(!r.errores.is_empty());
}

#[test]
fn ejecutar_la_suma_termina_bien_y_resume_el_estado() {
    let r = correr(&["ejecutar", &maquina(), &programa_de_ejemplo("suma.hex")]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("detenida: si"));
    assert!(r.salida.contains("instrucciones: 4"));
    assert!(r.salida.contains("ciclos: 7"));
    assert!(r.salida.contains("A=0x2c"));
    assert!(r.salida.contains("C=1"));
}

#[test]
fn la_traza_muestra_cada_instruccion() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--traza",
    ]);
    let lineas: Vec<&str> = r.salida.lines().take(4).collect();
    assert_eq!(lineas[0], "0000  LDI    2 ciclos");
    assert_eq!(lineas[2], "0004  ADD    2 ciclos");
    assert_eq!(lineas[3], "0005  HLT    1 ciclos");
}

#[test]
fn sin_traza_no_hay_lineas_de_instruccion() {
    let r = correr(&["ejecutar", &maquina(), &programa_de_ejemplo("suma.hex")]);
    assert!(!r.salida.contains("LDI"));
}

#[test]
fn el_bucle_infinito_termina_con_el_codigo_de_limite() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("bucle.hex"),
        "--limite",
        "50",
    ]);
    assert_eq!(r.codigo, SALIDA_LIMITE);
    assert!(r.errores.contains("limite de 50 instrucciones"));
    assert!(r.salida.contains("instrucciones: 50"));
}

#[test]
fn el_salto_aparece_marcado_en_la_traza() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("bucle.hex"),
        "--limite",
        "3",
        "--traza",
    ]);
    assert!(r.salida.contains("JZ"));
    assert!(r.salida.contains("salto"));
}

#[test]
fn un_fallo_de_ejecucion_devuelve_su_codigo_y_la_direccion() {
    // LDI A,1 ; ST A,[0x0010] escribe en la rom.
    let p = temporal("rom.hex", "20 01 50 10 00 F0");
    let r = correr(&["ejecutar", &maquina(), &p]);
    assert_eq!(r.codigo, SALIDA_ERROR_EJECUCION);
    assert!(r.errores.contains("error en 0002"));
    assert!(r.errores.contains("solo lectura"));
    assert!(r.salida.contains("detenida: no"));
}

#[test]
fn una_instruccion_invalida_es_un_fallo_de_ejecucion() {
    let p = temporal("invalida.hex", "80");
    let r = correr(&["ejecutar", &maquina(), &p]);
    assert_eq!(r.codigo, SALIDA_ERROR_EJECUCION);
    assert!(r.errores.contains("ninguna instruccion"));
}

#[test]
fn un_programa_ilegible_es_error_de_uso() {
    let r = correr(&["ejecutar", &maquina(), "/no/hay/programa.hex"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no se pudo leer"));
}

#[test]
fn un_programa_con_una_palabra_demasiado_grande_es_error_de_uso() {
    let p = temporal("grande.hex", "20 1FF");
    let r = correr(&["ejecutar", &maquina(), &p]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no cabe en una palabra de 8 bits"));
}

#[test]
fn un_programa_que_no_cabe_en_memoria_es_error_de_uso() {
    let p = temporal("fuera.hex", "00 00");
    let r = correr(&["ejecutar", &maquina(), &p, "--direccion", "0xF0FF"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
}

#[test]
fn se_puede_cargar_el_programa_en_la_ram() {
    let p = temporal("ram.hex", "20 03 F0");
    let r = correr(&["ejecutar", &maquina(), &p, "--direccion", "0x2000"]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("A=0x3"));
    assert!(r.salida.contains("pc: 2003"));
}

#[test]
fn argumentos_mal_escritos_son_error_de_uso() {
    let r = correr(&["ejecutar", &maquina()]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("<programa>"));
}

#[test]
fn los_codigos_de_salida_son_distintos() {
    let codigos = [
        SALIDA_OK,
        SALIDA_ERROR_EJECUCION,
        SALIDA_ERROR_USO,
        SALIDA_LIMITE,
    ];
    for (i, a) in codigos.iter().enumerate() {
        for b in &codigos[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn la_misma_ejecucion_da_la_misma_salida() {
    let a = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--traza",
    ]);
    let b = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--traza",
    ]);
    assert_eq!(a.salida, b.salida);
}

#[test]
fn pantalla_dibuja_lo_que_el_programa_escribio_en_video() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("pantalla.hex"),
        "--pantalla",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    let lineas: Vec<&str> = r
        .salida
        .lines()
        .skip_while(|l| !l.starts_with("pantalla pantalla:"))
        .collect();
    assert_eq!(lineas[1], format!("########{}", ".".repeat(56)));
    assert_eq!(lineas[2], format!("#{}", ".".repeat(63)));
    assert_eq!(lineas[5], ".".repeat(64));
    assert_eq!(lineas.len(), 1 + 32);
}

#[test]
fn sin_la_opcion_pantalla_no_se_dibuja_nada() {
    let r = correr(&["ejecutar", &maquina(), &programa_de_ejemplo("pantalla.hex")]);
    assert!(!r.salida.contains("pantalla pantalla:"));
}

#[test]
fn teclas_pulsa_en_el_primer_teclado() {
    // Lee el estado con un programa que no hace nada, pero la tecla queda esperando.
    let p = temporal("espera.hex", "F0");
    let r = correr(&["ejecutar", &maquina(), &p, "--teclas", "ab"]);
    assert_eq!(r.codigo, SALIDA_OK);
}

#[test]
fn teclas_en_una_maquina_sin_teclado_es_error_de_uso() {
    let texto = std::fs::read_to_string(maquina()).unwrap();
    let inicio = texto
        .find("[[dispositivo]]\nnombre       = \"teclado\"")
        .unwrap();
    let sin_teclado = &texto[..inicio];
    let m = temporal("sin_teclado.toml", sin_teclado);
    let p = temporal("espera2.hex", "F0");
    let r = correr(&["ejecutar", &m, &p, "--teclas", "a"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no tiene teclado"));
}

fn serie() -> String {
    raiz()
        .join("ejemplos/ejemplo-8-serie/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn programa_serie(nombre: &str) -> String {
    raiz()
        .join("ejemplos/ejemplo-8-serie/programas")
        .join(nombre)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn la_maquina_con_uart_es_valida() {
    let r = correr(&["validar", &serie()]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("ejemplo-8-serie"));
}

#[test]
fn hola_imprime_el_texto_transmitido_por_la_uart() {
    let r = correr(&["ejecutar", &serie(), &programa_serie("hola.hex")]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("uart serie:\nHola\n"));
}

#[test]
fn una_maquina_sin_transmision_no_imprime_la_seccion_uart() {
    let r = correr(&["ejecutar", &maquina(), &programa_de_ejemplo("suma.hex")]);
    assert!(!r.salida.contains("uart"));
}

#[test]
fn eco_devuelve_el_byte_entregado_con_serie() {
    let r = correr(&[
        "ejecutar",
        &serie(),
        &programa_serie("eco.hex"),
        "--serie",
        "Q",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("uart serie:\nQ\n"));
}

#[test]
fn serie_en_una_maquina_sin_uart_es_error_de_uso() {
    let p = temporal("espera3.hex", "F0");
    let r = correr(&["ejecutar", &maquina(), &p, "--serie", "x"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no tiene UART"));
}

#[test]
fn un_texto_con_salto_de_linea_no_agrega_otro() {
    let p = temporal("eco2.hex", "80 90 F0");
    let r = correr(&["ejecutar", &serie(), &p, "--serie", "\n"]);
    assert!(r.salida.contains("uart serie:\n\n"));
    assert!(!r.salida.contains("uart serie:\n\n\n"));
}

// --- Volcado de memoria (RF-EMU-024) ---

#[test]
fn volcar_imprime_el_rango_pedido() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        "0:6",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r
        .salida
        .contains("volcado desde 0000, 6 palabras:\n20 C8 24 64 31 F0\n"));
}

#[test]
fn volcar_a_un_archivo_lo_escribe_en_formato_cargable() {
    let destino = temporal("volcado.hex", "");
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        &format!("0:6:{destino}"),
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(!r.salida.contains("volcado desde"));
    let contenido = std::fs::read_to_string(&destino).unwrap();
    assert_eq!(contenido, "20 C8 24 64 31 F0\n");
    // Y ese archivo se puede ejecutar de nuevo con el mismo resultado.
    let otra = correr(&["ejecutar", &maquina(), &destino]);
    assert_eq!(otra.codigo, SALIDA_OK);
    assert!(otra.salida.contains("A=0x2c"));
}

#[test]
fn volcar_lo_que_el_programa_escribio_en_la_ram() {
    let p = temporal("escribe.hex", "20 42 50 00 20 F0");
    let r = correr(&["ejecutar", &maquina(), &p, "--volcar", "0x2000:2"]);
    assert!(r
        .salida
        .contains("volcado desde 2000, 2 palabras:\n42 00\n"));
}

#[test]
fn volcar_fuera_de_las_regiones_es_error_de_uso() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        "0xF800:4",
    ]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("error al volcar"));
}

#[test]
fn volcar_a_una_ruta_imposible_es_error_de_uso() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        "0:2:/no/existe/dir/x.hex",
    ]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no se pudo escribir"));
}

#[test]
fn se_vuelcan_varios_rangos_en_el_orden_pedido() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        "4:2",
        "--volcar",
        "0:1",
    ]);
    let a = r.salida.find("volcado desde 0004").unwrap();
    let b = r.salida.find("volcado desde 0000").unwrap();
    assert!(a < b);
}

#[test]
fn un_volcado_largo_se_parte_en_lineas_de_dieciseis() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--volcar",
        "0:40",
    ]);
    let lineas: Vec<&str> = r
        .salida
        .lines()
        .skip_while(|l| !l.starts_with("volcado"))
        .skip(1)
        .collect();
    assert_eq!(lineas.len(), 3);
    assert_eq!(lineas[0].split(' ').count(), 16);
    assert_eq!(lineas[2].split(' ').count(), 8);
}

// --- Interrupciones disparadas a mano (RF-EMU-040) y su registro (RF-EMU-039) ---

#[test]
fn interrumpir_ejecuta_el_manejador_y_vuelve() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("interrupcion.hex"),
        "--interrumpir",
        "0@8",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("C=0x99"));
    assert!(r.salida.contains("SP=0xefff"));
}

#[test]
fn sin_interrumpir_el_manejador_no_se_ejecuta() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("interrupcion.hex"),
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(r.salida.contains("C=0x0"));
    assert!(!r.salida.contains("interrupcion:"));
}

#[test]
fn el_resumen_lista_la_interrupcion_atendida() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("interrupcion.hex"),
        "--interrumpir",
        "0@8",
    ]);
    assert!(r.salida.contains("interrupcion: linea 0 en el ciclo"));
    assert!(r.salida.contains("manejador 0010"));
}

#[test]
fn interrumpir_una_linea_sin_vector_es_error_de_uso() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--interrumpir",
        "7@0",
    ]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("linea de interrupcion 7"));
}

#[test]
fn un_disparo_despues_del_final_no_se_levanta() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("suma.hex"),
        "--interrumpir",
        "0@100",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert!(!r.salida.contains("interrupcion:"));
}

#[test]
fn los_disparos_se_aplican_en_orden_de_instruccion_aunque_se_den_al_reves() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &programa_de_ejemplo("interrupcion.hex"),
        "--interrumpir",
        "0@9",
        "--interrumpir",
        "0@8",
    ]);
    assert_eq!(r.codigo, SALIDA_OK);
    assert_eq!(r.salida.matches("interrupcion: linea 0").count(), 2);
}
