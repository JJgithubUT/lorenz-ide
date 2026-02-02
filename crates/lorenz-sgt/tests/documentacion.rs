//! Los ejemplos de `docs/SGT.md` se compilan y se ejecutan (RF-ECO-027).
mod comun;
use comun::*;

struct Ejemplo {
    linea: usize,
    maquinas: Vec<&'static str>,
    espera_error: bool,
    fuente: String,
    esperas: Vec<(String, i128)>,
    error: Option<String>,
}

fn ejemplos() -> Vec<Ejemplo> {
    let ruta = format!("{}/../../docs/SGT.md", env!("CARGO_MANIFEST_DIR"));
    let texto = std::fs::read_to_string(ruta).unwrap();
    let mut v = Vec::new();
    let mut actual: Option<(usize, String, Vec<String>)> = None;
    for (n, l) in texto.lines().enumerate() {
        match &mut actual {
            None => {
                if let Some(etiqueta) = l.strip_prefix("```sgt") {
                    actual = Some((n + 1, etiqueta.to_string(), Vec::new()));
                }
            }
            Some((inicio, etiqueta, lineas)) => {
                if l.starts_with("```") {
                    let (maquinas, espera_error) = match etiqueta.as_str() {
                        "" => (vec!["risc-16", "sgt-16"], false),
                        ":sgt-16" => (vec!["sgt-16"], false),
                        "-error" => (vec!["risc-16"], true),
                        otra => panic!("docs/SGT.md:{inicio}: etiqueta desconocida `sgt{otra}`"),
                    };
                    let mut esperas = Vec::new();
                    let mut error = None;
                    for l in lineas.iter() {
                        if let Some(e) = l.strip_prefix("// espera: ") {
                            let (k, val) = e.split_once('=').unwrap();
                            esperas.push((k.trim().to_string(), val.trim().parse().unwrap()));
                        }
                        if let Some(e) = l.strip_prefix("// error: ") {
                            error = Some(e.trim().to_string());
                        }
                    }
                    v.push(Ejemplo {
                        linea: *inicio,
                        maquinas,
                        espera_error,
                        fuente: lineas.join("\n"),
                        esperas,
                        error,
                    });
                    actual = None;
                } else {
                    lineas.push(l.to_string());
                }
            }
        }
    }
    v
}

#[test]
fn hay_ejemplos_de_cada_clase() {
    let e = ejemplos();
    assert!(e.len() >= 15, "solo {} ejemplos", e.len());
    assert!(e.iter().any(|x| x.espera_error));
    assert!(e.iter().any(|x| x.maquinas == ["sgt-16"]));
}

#[test]
fn los_ejemplos_correctos_dan_lo_que_dicen() {
    for e in ejemplos().iter().filter(|e| !e.espera_error) {
        assert!(
            !e.esperas.is_empty(),
            "docs/SGT.md:{}: falta `// espera:`",
            e.linea
        );
        for m in &e.maquinas {
            let (g, _) = ejecutado(m, &e.fuente);
            for (k, v) in &e.esperas {
                assert_eq!(
                    g[k],
                    v & 0xFFFF,
                    "docs/SGT.md:{} en {m}: la variable `{k}` deberia valer {v}",
                    e.linea
                );
            }
        }
    }
}

#[test]
fn los_ejemplos_correctos_coinciden_con_el_interprete_de_referencia() {
    for e in ejemplos()
        .iter()
        .filter(|e| !e.espera_error && e.maquinas.len() == 2)
    {
        for m in &e.maquinas {
            igual_al_oraculo(m, &e.fuente);
        }
    }
}

#[test]
fn los_ejemplos_de_error_se_rechazan_con_el_texto_dicho() {
    for e in ejemplos().iter().filter(|e| e.espera_error) {
        let esperado = e
            .error
            .as_ref()
            .unwrap_or_else(|| panic!("docs/SGT.md:{}: falta `// error:`", e.linea));
        let m = e.maquinas[0];
        match compilar_en(m, &e.fuente) {
            Ok(_) => panic!("docs/SGT.md:{}: deberia fallar", e.linea),
            Err(msg) => assert!(
                msg.contains(esperado.as_str()),
                "docs/SGT.md:{}: el error no dice `{esperado}`:\n{msg}",
                e.linea
            ),
        }
    }
}
