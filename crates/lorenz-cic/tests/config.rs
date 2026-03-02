//! Lectura de la temporizacion de la descripcion (RF-CIC-003, RF-CIC-004, RF-CIC-015, RF-CIC-020).

use lorenz_cic::{cargar_config, Escritura, Prediccion, Reemplazo};

const BASE: &str = "[maquina]\nnombre = \"x\"\n";

#[test]
fn sin_secciones_de_tiempo_la_configuracion_esta_vacia() {
    let c = cargar_config(BASE).unwrap();
    assert!(c.pipeline.is_none() && c.cache.is_none() && c.latencias.is_empty());
    assert_eq!(c.latencia_de(0x1234), 1);
}

#[test]
fn un_pipeline_con_etapas_y_microoperaciones() {
    let t = format!(
        "{BASE}
[pipeline]
adelantamiento = false
prediccion = \"2-bits\"
[[pipeline.etapa]]
nombre = \"BUSCAR\"
hace = [\"leer la instruccion en pc\", \"pc = pc + 1\"]
[[pipeline.etapa]]
nombre = \"LEER\"
[[pipeline.etapa]]
nombre = \"EJEC\"
[[pipeline.etapa]]
nombre = \"MEM\"
[[pipeline.etapa]]
nombre = \"ESCR\"
"
    );
    let p = cargar_config(&t).unwrap().pipeline.unwrap();
    assert_eq!(p.etapas.len(), 5);
    assert_eq!(p.etapas[0].hace.len(), 2);
    assert!(!p.adelantamiento);
    assert_eq!(p.prediccion, Prediccion::DosBits);
    assert_eq!(
        (p.lectura, p.ejecucion, p.memoria, p.escritura, p.salto),
        (1, 2, 3, 4, 2)
    );
}

#[test]
fn los_papeles_de_las_etapas_se_pueden_nombrar() {
    let t = format!(
        "{BASE}
[pipeline]
etapas = [\"F\", \"D\", \"X1\", \"X2\", \"M\", \"W\"]
ejecucion = \"X1\"
salto = \"X2\"
"
    );
    let p = cargar_config(&t).unwrap().pipeline.unwrap();
    assert_eq!(p.ejecucion, 2);
    assert_eq!(p.salto, 3);
    assert_eq!(p.escritura, 5);
}

#[test]
fn errores_claros() {
    let e = |extra: &str| cargar_config(&format!("{BASE}{extra}")).unwrap_err();
    assert!(e("[pipeline]\netapas = [\"A\",\"B\"]\nprediccion = \"magica\"").contains("magica"));
    assert!(e("[pipeline]\netapas = [\"A\",\"B\"]\nlectura = \"Z\"").contains("`Z`"));
    assert!(e("[pipeline]\netapas = [\"A\",\"A\"]").contains("mismo nombre"));
    assert!(e("[cache]\ntamano = 10\nlinea = 4").contains("multiplo"));
    assert!(e("[cache]\ntamano = 8\nlinea = 2\nreemplazo = \"x\"").contains("reemplazo"));
    assert!(
        e("[[region]]\nnombre = \"r\"\ndesde = 0\nhasta = 1\nlatencia = 0").contains("al menos 1")
    );
}

#[test]
fn la_cache_y_las_latencias() {
    let t = format!(
        "{BASE}
[cache]
tamano = 64
linea = 4
vias = 2
reemplazo = \"fifo\"
escritura = \"escritura-directa\"
latencia_acierto = 2
[[region]]
nombre = \"rom\"
desde = 0
hasta = 255
latencia = 5
[[region]]
nombre = \"ram\"
desde = 256
hasta = 511
"
    );
    let c = cargar_config(&t).unwrap();
    let k = c.cache.clone().unwrap();
    assert_eq!(
        (k.tamano, k.linea, k.vias, k.latencia_acierto),
        (64, 4, 2, 2)
    );
    assert_eq!(k.reemplazo, Reemplazo::Fifo);
    assert_eq!(k.escritura, Escritura::EscrituraDirecta);
    assert_eq!(c.latencia_de(10), 5);
    assert_eq!(c.latencia_de(300), 1);
}

#[test]
fn el_emulador_funcional_acepta_las_mismas_secciones() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let con = format!(
        "{t}\n[pipeline]\netapas = [\"IF\",\"ID\",\"EX\"]\n[cache]\ntamano = 8\nlinea = 2\n"
    );
    assert!(lorenz_arch::cargar(&con).is_ok());
    assert!(cargar_config(&con).unwrap().pipeline.is_some());
}
