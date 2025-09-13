//! Pruebas de la declaracion de dispositivos en la descripcion de la arquitectura.

use lorenz_arch::modelo::TipoDispositivo;
use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn ejemplo() -> String {
    std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap()
}

fn con_dispositivo(bloque: &str) -> String {
    format!("{}\n{}\n", ejemplo(), bloque)
}

fn reglas(texto: &str) -> Vec<Regla> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(f)) => f.into_iter().map(|x| x.regla).collect(),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn un_temporizador_usa_el_periodo_declarado() {
    let m = cargar(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"reloj\"\ntipo = \"temporizador\"\nperiodo = 250\ninterrupcion = 0\n",
    ))
    .unwrap();
    let reloj = m.dispositivo("reloj").unwrap();
    assert_eq!(reloj.tipo, TipoDispositivo::Temporizador { periodo: 250 });
    assert_eq!(reloj.interrupcion, Some(0));
}

#[test]
fn sin_periodo_el_temporizador_usa_mil_ciclos() {
    let m = cargar(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"reloj\"\ntipo = \"temporizador\"\n",
    ))
    .unwrap();
    assert_eq!(
        m.dispositivo("reloj").unwrap().tipo,
        TipoDispositivo::Temporizador { periodo: 1000 }
    );
}

#[test]
fn un_periodo_de_cero_se_rechaza() {
    let r = reglas(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"reloj\"\ntipo = \"temporizador\"\nperiodo = 0\n",
    ));
    assert_eq!(r, [Regla::ValorFueraDeRango]);
}

#[test]
fn un_temporizador_con_una_linea_sin_vector_se_rechaza() {
    let r = reglas(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"reloj\"\ntipo = \"temporizador\"\ninterrupcion = 9\n",
    ));
    assert_eq!(r, [Regla::DispositivoInexistente]);
}

#[test]
fn un_tipo_de_dispositivo_desconocido_se_rechaza() {
    let r = reglas(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"x\"\ntipo = \"holograma\"\n",
    ));
    assert!(r.contains(&Regla::ValorFueraDeRango));
}

#[test]
fn la_pantalla_tiene_valores_por_omision() {
    let m = cargar(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"otra\"\ntipo = \"pantalla\"\n",
    ))
    .unwrap();
    assert_eq!(
        m.dispositivo("otra").unwrap().tipo,
        TipoDispositivo::Pantalla {
            ancho: 64,
            alto: 32,
            formato: "indice1".into()
        }
    );
}

#[test]
fn los_bloques_tienen_512_por_omision() {
    let m = cargar(&con_dispositivo(
        "[[dispositivo]]\nnombre = \"disco\"\ntipo = \"bloques\"\n",
    ))
    .unwrap();
    assert_eq!(
        m.dispositivo("disco").unwrap().tipo,
        TipoDispositivo::Bloques {
            tamano_bloque: 512,
            cantidad_bloques: 64
        }
    );
}

#[test]
fn una_region_que_cita_un_dispositivo_inexistente_se_rechaza() {
    let texto = ejemplo().replace("dispositivo = \"pantalla\"", "dispositivo = \"monitor\"");
    assert!(reglas(&texto).contains(&Regla::DispositivoInexistente));
}

fn bloques(extra: &str) -> String {
    con_dispositivo(&format!(
        "[[dispositivo]]\nnombre = \"disco\"\ntipo = \"bloques\"\n{extra}"
    ))
}

#[test]
fn los_bloques_aceptan_tamano_y_cantidad() {
    let m = cargar(&bloques("tamano_bloque = 16\ncantidad_bloques = 8\n")).unwrap();
    assert_eq!(
        m.dispositivo("disco").unwrap().tipo,
        TipoDispositivo::Bloques {
            tamano_bloque: 16,
            cantidad_bloques: 8
        }
    );
}

#[test]
fn un_tamano_de_bloque_cero_se_rechaza() {
    assert!(reglas(&bloques("tamano_bloque = 0\n")).contains(&Regla::ValorFueraDeRango));
}

#[test]
fn una_cantidad_de_bloques_cero_se_rechaza() {
    assert!(reglas(&bloques("cantidad_bloques = 0\n")).contains(&Regla::ValorFueraDeRango));
}

fn con_script(script: &str, extra: &str) -> String {
    con_dispositivo(&format!(
        "[[dispositivo]]\nnombre = \"eco\"\ntipo = \"generico\"\npuertos = {{ entrada = 0x30, salida = 0x31 }}\n{extra}script = \"{script}\"\n"
    ))
}

#[test]
fn un_script_valido_se_guarda_ya_analizado() {
    let m = cargar(&con_script("salida = valor + 1", "")).unwrap();
    match &m.dispositivo("eco").unwrap().tipo {
        TipoDispositivo::Generico { script, programa } => {
            assert_eq!(script, "salida = valor + 1");
            assert_eq!(programa.sentencias.len(), 1);
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn un_script_vacio_se_acepta() {
    assert!(cargar(&con_script("", "")).is_ok());
}

#[test]
fn un_script_mal_escrito_se_rechaza() {
    assert!(reglas(&con_script("salida = ", "")).contains(&Regla::SemanticaInvalida));
}

#[test]
fn un_script_que_cita_un_nombre_desconocido_se_rechaza() {
    assert!(reglas(&con_script("salida = fantasma", "")).contains(&Regla::SemanticaInvalida));
}

#[test]
fn un_script_puede_leer_valor_y_puerto() {
    assert!(cargar(&con_script("si puerto == 48: salida = valor", "")).is_ok());
}

#[test]
fn un_script_no_puede_asignar_a_valor() {
    assert!(reglas(&con_script("valor = 1", "")).contains(&Regla::SemanticaInvalida));
}

#[test]
fn un_script_puede_asignar_a_sus_puertos() {
    assert!(cargar(&con_script("entrada = 1; salida = 2", "")).is_ok());
}

#[test]
fn interrumpir_exige_declarar_una_interrupcion() {
    assert!(reglas(&con_script("interrumpir = 1", "")).contains(&Regla::SemanticaInvalida));
}

#[test]
fn interrumpir_se_acepta_con_una_interrupcion_declarada() {
    // la maquina de ejemplo ya declara los vectores 0 y 1
    let texto = con_script("interrumpir = 1", "interrupcion = 0\n");
    assert!(cargar(&texto).is_ok(), "{:?}", cargar(&texto).err());
}

#[test]
fn un_script_no_puede_tocar_la_memoria() {
    assert!(reglas(&con_script("salida = mem[0]", "")).contains(&Regla::SemanticaInvalida));
    assert!(reglas(&con_script("mem[0] = valor", "")).contains(&Regla::SemanticaInvalida));
}

#[test]
fn un_script_no_puede_usar_puertos_de_la_maquina_ni_detener() {
    assert!(reglas(&con_script("salida = in[1]", "")).contains(&Regla::SemanticaInvalida));
    assert!(reglas(&con_script("detener", "")).contains(&Regla::SemanticaInvalida));
}
