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

#[test]
fn un_hijo_hereda_la_temporizacion_del_padre_y_puede_cambiar_una_latencia() {
    let dir = std::env::temp_dir().join(format!("lorenz-cic-herencia-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let padre = std::fs::read_to_string("../../ejemplos/ejemplo-8-ciclos/maquina.toml").unwrap();
    std::fs::write(dir.join("padre.toml"), padre).unwrap();
    std::fs::write(
        dir.join("hijo.toml"),
        "[maquina]\nnombre = \"hijo\"\nhereda = \"padre.toml\"\n\n[[region]]\nnombre = \"ram\"\ndesde = 0x2000\nhasta = 0xEFFF\nacceso = \"rw\"\nlatencia = 9\n",
    )
    .unwrap();
    let c = lorenz_cic::cargar_config_archivo(dir.join("hijo.toml")).unwrap();
    assert!(
        c.pipeline.is_some() && c.cache.is_some(),
        "hereda las secciones"
    );
    assert_eq!(c.latencia_de(0x2100), 9, "redefine la latencia de la ram");
    assert_eq!(c.latencia_de(0x10), 2, "conserva la de la rom");
}

#[test]
fn las_restricciones_se_leen_con_enteros_y_decimales() {
    let c = lorenz_cic::cargar_config(
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 200\n\n\
         [[restriccion]]\nnombre = \"cpi bajo\"\nmetrica = \"cpi\"\nmaximo = 1.5\nminimo = 1\n",
    )
    .unwrap();
    assert_eq!(c.restricciones.len(), 2);
    assert_eq!(c.restricciones[0].nombre, "ciclos");
    assert_eq!(c.restricciones[0].maximo, Some(200.0));
    assert_eq!(c.restricciones[1].nombre, "cpi bajo");
    assert_eq!(c.restricciones[1].minimo, Some(1.0));
    assert_eq!(c.restricciones[1].maximo, Some(1.5));
}

#[test]
fn una_restriccion_con_metrica_desconocida_o_sin_limite_es_un_error() {
    let e = lorenz_cic::cargar_config("[[restriccion]]\nmetrica = \"calor\"\nmaximo = 1\n")
        .unwrap_err();
    assert!(e.contains("calor"), "{e}");
    let e = lorenz_cic::cargar_config("[[restriccion]]\nmetrica = \"ciclos\"\n").unwrap_err();
    assert!(e.contains("maximo"), "{e}");
}

#[test]
fn una_descripcion_sin_restricciones_da_una_lista_vacia() {
    assert!(lorenz_cic::cargar_config("")
        .unwrap()
        .restricciones
        .is_empty());
}

const REGIONES: &str = "[[region]]\nnombre = \"rom\"\ndesde = 0\nhasta = 0x7F\n\n\
                        [[region]]\nnombre = \"ram\"\ndesde = 0x80\nhasta = 0xFF\n\n";

#[test]
fn los_nucleos_resuelven_las_regiones_compartidas() {
    let c = lorenz_cic::cargar_config(&format!(
        "{REGIONES}[nucleos]\ncantidad = 4\ncompartida = [\"ram\"]\n"
    ))
    .unwrap();
    let n = c.nucleos.unwrap();
    assert_eq!(n.cantidad, 4);
    assert_eq!(n.compartidas.len(), 1);
    assert_eq!(
        (n.compartidas[0].desde, n.compartidas[0].hasta),
        (0x80, 0xFF)
    );
    assert!(n.es_compartida(0x80) && n.es_compartida(0xFF));
    assert!(!n.es_compartida(0x7F));
}

#[test]
fn sin_seccion_de_nucleos_hay_uno_solo() {
    assert!(lorenz_cic::cargar_config(REGIONES)
        .unwrap()
        .nucleos
        .is_none());
}

#[test]
fn los_nucleos_rechazan_cantidades_absurdas_y_regiones_que_no_existen() {
    let e = lorenz_cic::cargar_config("[nucleos]\ncantidad = 0\n").unwrap_err();
    assert!(e.contains("entre 1 y 64"), "{e}");
    let e = lorenz_cic::cargar_config("[nucleos]\ncantidad = 65\n").unwrap_err();
    assert!(e.contains("entre 1 y 64"), "{e}");
    let e = lorenz_cic::cargar_config(&format!(
        "{REGIONES}[nucleos]\ncantidad = 2\ncompartida = [\"vram\"]\n"
    ))
    .unwrap_err();
    assert!(e.contains("vram"), "{e}");
}

#[test]
fn varios_nucleos_sin_nada_compartido_es_un_error_pero_uno_solo_no() {
    let e = lorenz_cic::cargar_config("[nucleos]\ncantidad = 2\n").unwrap_err();
    assert!(e.contains("compartida"), "{e}");
    assert!(lorenz_cic::cargar_config("[nucleos]\ncantidad = 1\n").is_ok());
}
