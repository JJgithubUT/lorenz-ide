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
