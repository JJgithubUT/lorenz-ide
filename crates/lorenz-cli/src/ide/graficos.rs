//! Visor de la unidad grafica en tiempo real (RF-GPU-003): sesiones propias que corren un programa
//! de la CPU junto a la GPU y entregan el framebuffer como PNG para que la interfaz lo redibuje.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lorenz_emu::Emulador;
use lorenz_gpu::imagen::{a_png, inspeccionar};
use lorenz_gpu::{cargar_config_archivo, SistemaGrafico};
use lorenz_web::{Json, Peticion, Respuesta, Router};

use super::api::{cuerpo, error, num_de, ok, texto_de};
use super::sesion::Ide;

/// Sesiones graficas simultaneas como maximo.
pub const MAX_GRAFICAS: usize = 4;
/// Instrucciones que una sola peticion puede ejecutar.
const MAX_AVANCE: u64 = 5_000_000;

struct Grafica {
    sistema: SistemaGrafico,
    instrucciones: u64,
}

#[derive(Default)]
struct Graficas {
    sesiones: Mutex<BTreeMap<u32, Arc<Mutex<Grafica>>>>,
    siguiente: Mutex<u32>,
}

fn abrir(ide: &Ide, maquina_rel: &str, programa_rel: &str) -> Result<Grafica, String> {
    let rm = ide.ruta_texto(maquina_rel)?;
    let rp = ide.ruta_texto(programa_rel)?;
    let maquina = lorenz_arch::cargar_archivo(&rm).map_err(|e| format!("{maquina_rel}: {e}"))?;
    let config = cargar_config_archivo(&rm)
        .map_err(|e| format!("{maquina_rel}: {e}"))?
        .ok_or_else(|| {
            format!("{maquina_rel}: la descripcion no declara una unidad grafica (falta [gpu])")
        })?;
    let inicio = maquina.arranque;
    let segmentos = if crate::ensamblar::es_fuente(&rp) {
        let (o, _) = crate::ensamblar::ensamblar_ejecutable(&maquina, &rp)?;
        o.segmentos
            .iter()
            .map(|s| (s.direccion, s.palabras.clone()))
            .collect::<Vec<_>>()
    } else {
        crate::ensamblar::cargar_segmentos(&maquina, &rp, inicio)?
    };
    ide.permitir_maquina(&maquina)?;
    let mut emu = Emulador::nuevo(maquina).map_err(|e| format!("{maquina_rel}: {e}"))?;
    lorenz_emu::programa::precargar_regiones(&mut emu, &crate::ordenes::carpeta_de(&rm))
        .map_err(|e| format!("{maquina_rel}: {e}"))?;
    for (desde, palabras) in &segmentos {
        emu.cargar_programa(*desde, palabras)
            .map_err(|e| format!("{programa_rel}: {e}"))?;
    }
    emu.estado_mut().fijar_pc(inicio);
    let sistema = SistemaGrafico::nuevo(emu, config).map_err(|e| format!("{maquina_rel}: {e}"))?;
    Ok(Grafica {
        sistema,
        instrucciones: 0,
    })
}

fn estado(g: &Grafica, id: Option<u32>) -> Json {
    let gpu = g.sistema.gpu();
    let m = gpu.metricas();
    let s = gpu.superficie();
    Json::objeto([
        ("id", id.map_or(Json::Nulo, |i| u64::from(i).into())),
        ("ancho", u64::from(s.ancho()).into()),
        ("alto", u64::from(s.alto()).into()),
        ("instrucciones", g.instrucciones.into()),
        ("ciclos_cpu", g.sistema.ciclos_cpu().into()),
        ("detenida", g.sistema.emulador().estado().detenida().into()),
        ("pc", g.sistema.emulador().estado().pc().into()),
        (
            "metricas",
            Json::objeto([
                ("ciclos_ocupada", m.ciclos_ocupada.into()),
                ("ciclos_totales", m.ciclos_totales.into()),
                ("utilizacion", Json::Num(m.utilizacion())),
                ("comandos", m.comandos.into()),
                ("triangulos", m.triangulos.into()),
                ("lotes", m.lotes.into()),
                ("pixeles_escritos", m.pixeles_escritos.into()),
                ("cola_maxima", (m.cola_maxima as u64).into()),
                ("rechazados", m.rechazados.into()),
            ]),
        ),
    ])
}

fn con_grafica(
    gs: &Graficas,
    p: &Peticion,
    f: impl FnOnce(&mut Grafica) -> Respuesta,
) -> Respuesta {
    let Some(id) = p.parametro("id").and_then(|i| i.parse::<u32>().ok()) else {
        return error(400, "identificador de sesion grafica invalido");
    };
    let Some(s) = gs.sesiones.lock().ok().and_then(|m| m.get(&id).cloned()) else {
        return error(404, format!("no existe la sesion grafica {id}"));
    };
    let r = match s.lock() {
        Ok(mut g) => f(&mut g),
        Err(_) => error(500, "la sesion grafica quedo inconsistente; cierrala"),
    };
    r
}

pub fn rutas(router: Router, ide: Arc<Ide>) -> Router {
    let gs = Arc::new(Graficas::default());
    let (g1, g2, g3, g4, g5) = (gs.clone(), gs.clone(), gs.clone(), gs.clone(), gs);
    router
        .post("/api/gpu", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, pr) = match (texto_de(&j, "maquina"), texto_de(&j, "programa")) {
                (Ok(m), Ok(pr)) => (m, pr),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let Ok(mut todas) = g1.sesiones.lock() else {
                return error(500, "estado corrupto");
            };
            if todas.len() >= MAX_GRAFICAS {
                return error(
                    429,
                    format!("hay demasiadas sesiones graficas (maximo {MAX_GRAFICAS})"),
                );
            }
            let g = match abrir(&ide, m, pr) {
                Ok(g) => g,
                Err(e) => return error(422, e),
            };
            let Ok(mut sig) = g1.siguiente.lock() else {
                return error(500, "estado corrupto");
            };
            *sig += 1;
            let id = *sig;
            let e = estado(&g, Some(id));
            todas.insert(id, Arc::new(Mutex::new(g)));
            ok(e)
        })
        .get("/api/gpu/:id/estado", move |p| {
            con_grafica(&g2, p, |g| ok(estado(g, None)))
        })
        .post("/api/gpu/:id/avanzar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let n = match num_de(&j, "n", 1000) {
                Ok(n) => n.min(MAX_AVANCE),
                Err(r) => return r,
            };
            con_grafica(&g3, p, |g| {
                let mut hechas = 0;
                while hechas < n && !g.sistema.emulador().estado().detenida() {
                    match g.sistema.paso() {
                        Ok(Some(_)) => {
                            hechas += 1;
                            g.instrucciones += 1;
                        }
                        Ok(None) => break,
                        Err(e) => return error(422, format!("fallo durante la ejecucion: {e}")),
                    }
                }
                if g.sistema.emulador().estado().detenida() {
                    g.sistema.terminar_gpu(4_000_000_000);
                }
                ok(estado(g, None))
            })
        })
        .get("/api/gpu/:id/imagen", move |p| {
            let factor = p
                .consulta("ampliar")
                .and_then(|f| f.parse::<u32>().ok())
                .unwrap_or(1)
                .clamp(1, 16);
            con_grafica(&g4, p, |g| {
                let png = a_png(g.sistema.gpu().superficie(), factor);
                Respuesta::nueva(200, "image/png", png).con_cabecera("Cache-Control", "no-store")
            })
        })
        .get("/api/gpu/:id/pixel", move |p| {
            let (Some(x), Some(y)) = (
                p.consulta("x").and_then(|v| v.parse::<i64>().ok()),
                p.consulta("y").and_then(|v| v.parse::<i64>().ok()),
            ) else {
                return error(400, "`x` e `y` deben ser enteros");
            };
            con_grafica(&g5, p, |g| {
                ok(Json::objeto([(
                    "texto",
                    inspeccionar(g.sistema.gpu().superficie(), x, y).into(),
                )]))
            })
        })
}
