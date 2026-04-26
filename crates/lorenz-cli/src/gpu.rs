//! `lorenz gpu`: ejecutar un programa o un guion de comandos en la unidad grafica
//! (RF-GPU-004, RF-GPU-017, RF-GPU-022).

use std::io::Write;

use lorenz_emu::Emulador;
use lorenz_gpu::imagen::{a_png, a_ppm, a_texto, inspeccionar};
use lorenz_gpu::{cargar_config_archivo, leer_guion, Gpu, Metricas, SistemaGrafico};

use crate::ciclos::cargar_con_lineas;
use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesGpu {
    pub limite: u64,
    pub direccion: u64,
    /// Archivo `.png` o `.ppm` donde volcar el framebuffer.
    pub imagen: Option<String>,
    /// Factor entero de ampliacion de la imagen.
    pub ampliar: u32,
    /// Pixeles a describir.
    pub pixeles: Vec<(i64, i64)>,
    /// Dibujar el resultado con caracteres.
    pub texto: bool,
}

impl Default for OpcionesGpu {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            imagen: None,
            ampliar: 1,
            pixeles: Vec::new(),
            texto: false,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

/// Tope de ciclos de la unidad al terminar lo que quedo en su cola.
const LIMITE_CICLOS_GPU: u64 = 4_000_000_000;

pub fn texto_de_metricas(m: &Metricas, ciclos_cpu: Option<u64>) -> String {
    let mut s = String::new();
    if let Some(c) = ciclos_cpu {
        s.push_str(&format!("ciclos de la CPU: {c}\n"));
    }
    s.push_str(&format!(
        "ciclos de la GPU: {} ocupada de {} ({:.1}% de uso)\n",
        m.ciclos_ocupada,
        m.ciclos_totales,
        m.utilizacion() * 100.0
    ));
    s.push_str(&format!(
        "comandos: {} ({} triangulos, {} lotes)\n",
        m.comandos, m.triangulos, m.lotes
    ));
    s.push_str(&format!("pixeles escritos: {}\n", m.pixeles_escritos));
    s.push_str(&format!("cola: maximo {} comandos", m.cola_maxima));
    if m.rechazados > 0 {
        s.push_str(&format!(", {} rechazados por cola llena", m.rechazados));
    }
    s.push('\n');
    s
}

pub fn gpu_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesGpu,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let uso = |errores: &mut dyn Write, m: String| {
        escribir(errores, &format!("{m}\n"));
        SALIDA_ERROR_USO
    };
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    let config = match cargar_config_archivo(ruta_maquina) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return uso(
                errores,
                format!(
                    "{ruta_maquina}: la descripcion no declara una unidad grafica (falta [gpu])"
                ),
            )
        }
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };

    let (gpu_final, ciclos_cpu, detenida): (Gpu, Option<u64>, bool);
    if ruta_programa.ends_with(".gpu") {
        let texto = match std::fs::read_to_string(ruta_programa) {
            Ok(t) => t,
            Err(e) => return uso(errores, format!("{ruta_programa}: {e}")),
        };
        let comandos = match leer_guion(&texto) {
            Ok(c) => c,
            Err(e) => return uso(errores, format!("{ruta_programa}: {e}")),
        };
        let mut gpu = Gpu::nueva(config);
        for c in comandos {
            // Un guion no tiene CPU que espere: si la cola esta llena, deja correr la unidad.
            while gpu.en_cola() >= gpu.config().cola {
                gpu.avanzar(1);
            }
            let _ = gpu.enviar(c);
        }
        gpu.vaciar(LIMITE_CICLOS_GPU);
        (gpu_final, ciclos_cpu, detenida) = (gpu, None, true);
    } else {
        let segmentos = match cargar_con_lineas(&maquina, ruta_programa, opciones.direccion) {
            Ok((s, _, _)) => s,
            Err(e) => return uso(errores, e),
        };
        let mut emu = match Emulador::nuevo(maquina) {
            Ok(e) => e,
            Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
        };
        for (dir, p) in &segmentos {
            if let Err(e) = emu.cargar_programa(*dir, p) {
                return uso(errores, format!("{ruta_programa}: {e}"));
            }
        }
        let mut sistema = match SistemaGrafico::nuevo(emu, config) {
            Ok(s) => s,
            Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
        };
        let mut limite_alcanzado = false;
        match sistema.ejecutar(opciones.limite) {
            Ok(_) => {}
            Err(lorenz_emu::ErrorEjecucion::LimiteDeInstrucciones { .. }) => {
                limite_alcanzado = true
            }
            Err(e) => {
                escribir(errores, &format!("fallo durante la ejecucion: {e}\n"));
                return SALIDA_ERROR_EJECUCION;
            }
        }
        sistema.terminar_gpu(LIMITE_CICLOS_GPU);
        ciclos_cpu = Some(sistema.ciclos_cpu());
        detenida = !limite_alcanzado;
        gpu_final = sistema.gpu().clone();
    }

    escribir(
        salida,
        &texto_de_metricas(&gpu_final.metricas(), ciclos_cpu),
    );
    let s = gpu_final.superficie();
    for (x, y) in &opciones.pixeles {
        escribir(salida, &format!("{}\n", inspeccionar(s, *x, *y)));
    }
    if opciones.texto {
        escribir(salida, &a_texto(s));
    }
    if let Some(ruta) = &opciones.imagen {
        let bytes = if ruta.ends_with(".ppm") {
            a_ppm(s, opciones.ampliar)
        } else {
            a_png(s, opciones.ampliar)
        };
        if let Err(e) = std::fs::write(ruta, bytes) {
            return uso(errores, format!("{ruta}: {e}"));
        }
        escribir(
            salida,
            &format!(
                "imagen: {ruta} ({}x{})\n",
                s.ancho() * opciones.ampliar.max(1),
                s.alto() * opciones.ampliar.max(1)
            ),
        );
    }
    if detenida {
        SALIDA_OK
    } else {
        escribir(
            errores,
            &format!(
                "se alcanzo el limite de {} instrucciones\n",
                opciones.limite
            ),
        );
        SALIDA_LIMITE
    }
}
