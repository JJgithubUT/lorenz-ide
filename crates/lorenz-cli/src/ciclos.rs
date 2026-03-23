//! La orden `ciclos`: ejecuta un programa con la temporizacion de la descripcion (pipeline,
//! prediccion de saltos, cache y latencias) y cuenta ciclos (RF-CIC-001, RF-CIC-002,
//! RF-CIC-012, RF-CIC-025 a RF-CIC-032).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;

use lorenz_cic::{cargar_config_archivo, Metricas, Simulador};
use lorenz_emu::Emulador;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesCiclos {
    pub limite: u64,
    pub direccion: u64,
    /// Mostrar el pipeline de los primeros N ciclos.
    pub vista: Option<u64>,
    /// Dibujar el diagrama de tiempos de las primeras N instrucciones.
    pub diagrama: Option<usize>,
    pub perfil: bool,
    pub volcar_cache: bool,
    /// Mapa de calor sobre el fuente, en texto (RF-CIC-032).
    pub calor: bool,
    /// Mapa de calor sobre el fuente, en una pagina HTML.
    pub calor_html: Option<String>,
    /// Anadir la medicion a este archivo de historial (RF-CIC-030).
    pub historial: Option<String>,
    pub etiqueta: Option<String>,
    /// Repetir la simulacion cada vez que cambien los archivos (RF-CIC-028).
    pub vigilar: bool,
}

impl Default for OpcionesCiclos {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            vista: None,
            diagrama: None,
            perfil: false,
            volcar_cache: false,
            calor: false,
            calor_html: None,
            historial: None,
            etiqueta: None,
            vigilar: false,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

fn porcentaje(x: Option<f64>) -> String {
    x.map_or("-".to_string(), |v| format!("{:.1} %", v * 100.0))
}

/// Huella estable de una descripcion: FNV-1a sobre su TOML ya mezclado con la herencia, de modo
/// que los comentarios y el espaciado no cambian la version (RF-CIC-030).
pub fn version_de(ruta_maquina: &str) -> Result<String, String> {
    let t = lorenz_arch::mezclar_archivo(ruta_maquina).map_err(|e| e.to_string())?;
    let texto = toml::to_string(&t).map_err(|e| e.to_string())?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in texto.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    Ok(format!("{h:016x}"))
}

/// Una ejecucion terminada, con todo lo que hace falta para informar de ella.
pub struct Corrida {
    pub sim: Simulador,
    pub nombre_maquina: String,
    pub version: String,
    pub bytes_de_codigo: u64,
    /// `(direccion, palabras, linea)` de cada tramo con linea de fuente conocida.
    pub lineas: Vec<(u64, u64, usize)>,
    pub fuente: Option<String>,
}

/// Preparar y ejecutar un programa. El error lleva el codigo de salida y el mensaje.
pub fn simular(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesCiclos,
) -> Result<Corrida, (i32, String)> {
    let uso = |m: String| (SALIDA_ERROR_USO, m);
    let maquina = lorenz_arch::cargar_archivo(ruta_maquina)
        .map_err(|e| uso(format!("{ruta_maquina}: {e}")))?;
    let config =
        cargar_config_archivo(ruta_maquina).map_err(|e| uso(format!("{ruta_maquina}: {e}")))?;
    let version = version_de(ruta_maquina).map_err(uso)?;
    let (segmentos, lineas, fuente) =
        cargar_con_lineas(&maquina, ruta_programa, opciones.direccion).map_err(uso)?;
    let bits_palabra = u64::from(maquina.bits_palabra);
    let palabras: u64 = segmentos.iter().map(|(_, p)| p.len() as u64).sum();
    let nombre_maquina = maquina.nombre.clone();
    let mut emu = Emulador::nuevo(maquina).map_err(|e| uso(format!("{ruta_maquina}: {e}")))?;
    for (dir, p) in &segmentos {
        emu.cargar_programa(*dir, p)
            .map_err(|e| uso(format!("{ruta_programa}: {e}")))?;
    }
    let mut sim = Simulador::nuevo(emu, config).map_err(|e| uso(format!("{ruta_maquina}: {e}")))?;
    sim.ejecutar(opciones.limite).map_err(|e| {
        (
            SALIDA_ERROR_EJECUCION,
            format!("fallo durante la ejecucion: {e}"),
        )
    })?;
    Ok(Corrida {
        sim,
        nombre_maquina,
        version,
        bytes_de_codigo: (palabras * bits_palabra).div_ceil(8),
        lineas,
        fuente,
    })
}

pub(crate) type Segmentos = Vec<(u64, Vec<u64>)>;
pub(crate) type Lineas = Vec<(u64, u64, usize)>;

/// Como `cargar_segmentos`, pero dice tambien a que linea del fuente corresponde cada tramo y
/// devuelve el texto del fuente, si lo hay.
pub(crate) fn cargar_con_lineas(
    maquina: &lorenz_arch::modelo::Maquina,
    ruta: &str,
    inicio: u64,
) -> Result<(Segmentos, Lineas, Option<String>), String> {
    if crate::compilar::es_sgt(ruta) {
        let (c, texto) = crate::compilar::compilar_archivo(maquina, ruta, None)?;
        let fin = c
            .objeto
            .segmentos
            .iter()
            .map(|s| s.direccion + s.palabras.len() as u64);
        let fin = fin.max().unwrap_or(0);
        let mut lineas = Vec::new();
        for (i, (d, l)) in c.lineas_fuente.iter().enumerate() {
            let hasta = c.lineas_fuente.get(i + 1).map_or(fin, |(n, _)| *n);
            lineas.push((*d, hasta.saturating_sub(*d), *l as usize));
        }
        let seg = c
            .objeto
            .segmentos
            .into_iter()
            .map(|s| (s.direccion, s.palabras))
            .collect();
        return Ok((seg, lineas, Some(texto)));
    }
    if crate::ensamblar::es_fuente(ruta) {
        let (objeto, _) = crate::ensamblar::ensamblar_ejecutable(maquina, ruta)?;
        let lineas = objeto
            .mapa
            .iter()
            .filter(|e| e.archivo == ruta)
            .map(|e| (e.direccion, e.longitud as u64, e.linea))
            .collect();
        let texto = std::fs::read_to_string(ruta).ok();
        let seg = objeto
            .segmentos
            .into_iter()
            .map(|s| (s.direccion, s.palabras))
            .collect();
        return Ok((seg, lineas, texto));
    }
    let seg = crate::ensamblar::cargar_segmentos(maquina, ruta, inicio)?;
    Ok((seg, Vec::new(), None))
}

/// Las metricas como texto, una por linea.
pub fn texto_de_metricas(m: &Metricas, bytes_de_codigo: Option<u64>) -> String {
    let mut s = String::new();
    s.push_str(&format!("ciclos: {}\n", m.ciclos));
    s.push_str(&format!("instrucciones: {}\n", m.instrucciones));
    s.push_str(&format!(
        "ciclos por instruccion: {}\n",
        m.cpi().map_or("-".to_string(), |c| format!("{c:.3}"))
    ));
    s.push_str(&format!(
        "esperas: {} por datos, {} estructurales, {} por saltos\n",
        m.esperas_datos, m.esperas_estructurales, m.esperas_control
    ));
    if m.esperas_bus > 0 {
        s.push_str(&format!(
            "esperas por el bus compartido: {}\n",
            m.esperas_bus
        ));
    }
    if m.saltos > 0 {
        s.push_str(&format!(
            "saltos: {} ({} acertados, {} fallados, {} instrucciones descartadas)\n",
            m.saltos,
            m.saltos_acertados,
            m.saltos_fallados(),
            m.descartadas
        ));
    }
    if let Some(c) = &m.cache {
        s.push_str(&format!(
            "cache: lecturas {} aciertos / {} fallos, escrituras {} aciertos / {} fallos, tasa {}\n",
            c.aciertos_lectura,
            c.fallos_lectura,
            c.aciertos_escritura,
            c.fallos_escritura,
            porcentaje(c.tasa_de_aciertos())
        ));
    }
    s.push_str(&format!("uso del bus: {}\n", porcentaje(m.uso_del_bus())));
    if let Some(b) = bytes_de_codigo {
        s.push_str(&format!("tamano del codigo: {b} bytes\n"));
    }
    s
}

/// Ciclos de cada linea del fuente, de la primera a la ultima (RF-CIC-032).
pub fn ciclos_por_linea(c: &Corrida) -> Vec<(usize, u64)> {
    let mut por_linea: std::collections::BTreeMap<usize, u64> = std::collections::BTreeMap::new();
    for f in c.sim.perfil() {
        let linea = c
            .lineas
            .iter()
            .find(|(d, n, _)| f.direccion >= *d && f.direccion < d + (*n).max(1))
            .map(|(_, _, l)| *l);
        if let Some(l) = linea {
            *por_linea.entry(l).or_insert(0) += f.ciclos;
        }
    }
    por_linea.into_iter().collect()
}

/// El fuente con los ciclos de cada linea y una barra proporcional.
pub fn mapa_de_calor(c: &Corrida) -> Option<String> {
    let fuente = c.fuente.as_ref()?;
    let ciclos = ciclos_por_linea(c);
    let maximo = ciclos.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
    let total: u64 = ciclos.iter().map(|(_, n)| *n).sum();
    let mut s = format!("# mapa de calor: {total} ciclos atribuidos al fuente\n");
    for (i, texto) in fuente.lines().enumerate() {
        let n = ciclos.iter().find(|(l, _)| *l == i + 1).map(|(_, n)| *n);
        match n {
            Some(n) => {
                let barra = "#".repeat((n * 10).div_ceil(maximo) as usize);
                s.push_str(&format!("{n:>8} {barra:<10} {:>4}| {texto}\n", i + 1));
            }
            None => s.push_str(&format!("{:>8} {:<10} {:>4}| {texto}\n", "", "", i + 1)),
        }
    }
    Some(s)
}

fn escapar(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// El mismo mapa como pagina HTML autonoma, con el fondo de cada linea mas rojo cuanto mas cuesta.
pub fn mapa_de_calor_html(c: &Corrida) -> Option<String> {
    let fuente = c.fuente.as_ref()?;
    let ciclos = ciclos_por_linea(c);
    let maximo = ciclos.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
    let mut s = String::from(
        "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><title>Mapa de calor</title>\n\
<style>body{font:14px ui-monospace,monospace;margin:1rem}table{border-collapse:collapse}\
td{padding:0 .6rem;white-space:pre}td.n{text-align:right;color:#555}td.c{text-align:right}</style>\n\
<table>\n",
    );
    for (i, texto) in fuente.lines().enumerate() {
        let n = ciclos.iter().find(|(l, _)| *l == i + 1).map(|(_, n)| *n);
        let (fondo, cuenta) = match n {
            Some(n) => (
                format!(
                    "background:rgba(220,60,40,{:.2})",
                    0.08 + 0.72 * n as f64 / maximo as f64
                ),
                n.to_string(),
            ),
            None => (String::new(), String::new()),
        };
        s.push_str(&format!(
            "<tr style=\"{fondo}\"><td class=\"n\">{}</td><td class=\"c\">{cuenta}</td><td>{}</td></tr>\n",
            i + 1,
            escapar(texto)
        ));
    }
    s.push_str("</table>\n");
    Some(s)
}

/// Informe completo de una corrida segun las opciones.
fn informar(
    c: &mut Corrida,
    opciones: &OpcionesCiclos,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    if let Some(n) = opciones.vista {
        for ciclo in 0..n.min(c.sim.ciclos()) {
            match c.sim.vista(ciclo) {
                Ok(v) => escribir(salida, &format!("{}\n", v.texto())),
                Err(e) => {
                    escribir(errores, &format!("{e}\n"));
                    return SALIDA_ERROR_EJECUCION;
                }
            }
        }
    }
    if let Some(n) = opciones.diagrama {
        escribir(salida, &c.sim.diagrama(0, n));
    }
    escribir(
        salida,
        &texto_de_metricas(&c.sim.metricas(), Some(c.bytes_de_codigo)),
    );
    if opciones.perfil {
        escribir(salida, "# direccion instruccion ejecuciones ciclos\n");
        let mut filas = c.sim.perfil();
        filas.sort_by_key(|f| std::cmp::Reverse(f.ciclos));
        for f in filas {
            escribir(
                salida,
                &format!(
                    "{:#06x} {} {} {}\n",
                    f.direccion, f.mnemonico, f.ejecuciones, f.ciclos
                ),
            );
        }
    }
    if opciones.volcar_cache {
        match c.sim.cache() {
            Some(k) => escribir(salida, &k.volcar()),
            None => escribir(salida, "la descripcion no declara cache\n"),
        }
    }
    if opciones.calor {
        match mapa_de_calor(c) {
            Some(t) => escribir(salida, &t),
            None => {
                escribir(
                    errores,
                    "el programa no tiene fuente: no hay mapa de calor\n",
                );
                return SALIDA_ERROR_USO;
            }
        }
    }
    if let Some(ruta) = &opciones.calor_html {
        let Some(html) = mapa_de_calor_html(c) else {
            escribir(
                errores,
                "el programa no tiene fuente: no hay mapa de calor\n",
            );
            return SALIDA_ERROR_USO;
        };
        if let Err(e) = std::fs::write(ruta, html) {
            escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
            return SALIDA_ERROR_USO;
        }
        escribir(salida, &format!("{ruta}: mapa de calor escrito\n"));
    }
    SALIDA_OK
}

/// Una vuelta de `--vigilar`: vuelve a simular si cambio la descripcion o el programa.
#[derive(Debug, Default)]
pub struct VigilanteCiclos {
    ultima: Option<u64>,
}

impl VigilanteCiclos {
    fn huella(ruta_maquina: &str, ruta_programa: &str) -> Option<u64> {
        let mut h = DefaultHasher::new();
        for a in lorenz_arch::cadena_de_herencia(Path::new(ruta_maquina)) {
            std::fs::read_to_string(a).ok()?.hash(&mut h);
        }
        std::fs::read(ruta_programa).ok()?.hash(&mut h);
        // Los modulos de un programa SGT y los `.incluir` viven en la misma carpeta.
        if let Some(dir) = Path::new(ruta_programa).parent() {
            let mut nombres: Vec<_> = std::fs::read_dir(dir)
                .ok()?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| matches!(x, "sgt" | "s" | "inc"))
                })
                .collect();
            nombres.sort();
            for n in nombres {
                std::fs::read(&n).ok()?.hash(&mut h);
            }
        }
        Some(h.finish())
    }

    /// Simula otra vez solo si algo cambio. Devuelve `None` si no hubo cambios.
    pub fn pasar(
        &mut self,
        ruta_maquina: &str,
        ruta_programa: &str,
        opciones: &OpcionesCiclos,
    ) -> Option<Result<Corrida, (i32, String)>> {
        let h = Self::huella(ruta_maquina, ruta_programa);
        if h.is_some() && h == self.ultima {
            return None;
        }
        self.ultima = h;
        Some(simular(ruta_maquina, ruta_programa, opciones))
    }
}

pub fn ciclos_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesCiclos,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    if opciones.vigilar {
        let mut v = VigilanteCiclos::default();
        loop {
            match v.pasar(ruta_maquina, ruta_programa, opciones) {
                None => {}
                Some(Err((_, e))) => escribir(errores, &format!("{e}\n")),
                Some(Ok(mut c)) => {
                    escribir(
                        salida,
                        &format!("--- {} ({})\n", c.nombre_maquina, c.version),
                    );
                    informar(&mut c, opciones, salida, errores);
                }
            }
            let _ = salida.flush();
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    let mut c = match simular(ruta_maquina, ruta_programa, opciones) {
        Ok(c) => c,
        Err((codigo, e)) => {
            escribir(errores, &format!("{e}\n"));
            return codigo;
        }
    };
    let codigo = informar(&mut c, opciones, salida, errores);
    if codigo != SALIDA_OK {
        return codigo;
    }
    if let Some(ruta) = &opciones.historial {
        if let Err(e) =
            crate::historial_ciclos::anadir(ruta, &c, ruta_programa, opciones.etiqueta.as_deref())
        {
            escribir(errores, &format!("{ruta}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
        escribir(
            salida,
            &format!("{ruta}: medicion anadida (version {})\n", c.version),
        );
    }
    if c.sim.detenida() {
        SALIDA_OK
    } else {
        escribir(
            salida,
            &format!(
                "limite de {} instrucciones alcanzado\n",
                c.sim.metricas().instrucciones
            ),
        );
        SALIDA_LIMITE
    }
}

/// Compara las metricas del mismo programa en dos descripciones (RF-CIC-029).
pub fn comparar_ciclos_cli(
    maquina_a: &str,
    maquina_b: &str,
    ruta_programa: &str,
    opciones: &OpcionesCiclos,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let mut corrida = |m: &str| match simular(m, ruta_programa, opciones) {
        Ok(c) => Ok(c),
        Err((codigo, e)) => {
            escribir(errores, &format!("{e}\n"));
            Err(codigo)
        }
    };
    let a = match corrida(maquina_a) {
        Ok(c) => c,
        Err(c) => return c,
    };
    let b = match corrida(maquina_b) {
        Ok(c) => c,
        Err(c) => return c,
    };
    escribir(salida, &tabla_de_comparacion(&a, &b));
    SALIDA_OK
}

/// Dos corridas lado a lado, con la diferencia de cada cifra.
pub fn tabla_de_comparacion(a: &Corrida, b: &Corrida) -> String {
    let (ma, mb) = (a.sim.metricas(), b.sim.metricas());
    let mut filas: Vec<(&str, f64, f64)> = vec![
        ("ciclos", ma.ciclos as f64, mb.ciclos as f64),
        (
            "instrucciones",
            ma.instrucciones as f64,
            mb.instrucciones as f64,
        ),
        (
            "ciclos por instruccion",
            ma.cpi().unwrap_or(0.0),
            mb.cpi().unwrap_or(0.0),
        ),
        (
            "esperas por datos",
            ma.esperas_datos as f64,
            mb.esperas_datos as f64,
        ),
        (
            "esperas estructurales",
            ma.esperas_estructurales as f64,
            mb.esperas_estructurales as f64,
        ),
        (
            "esperas por saltos",
            ma.esperas_control as f64,
            mb.esperas_control as f64,
        ),
        (
            "saltos fallados",
            ma.saltos_fallados() as f64,
            mb.saltos_fallados() as f64,
        ),
        (
            "tamano del codigo (bytes)",
            a.bytes_de_codigo as f64,
            b.bytes_de_codigo as f64,
        ),
    ];
    if ma.esperas_bus > 0 || mb.esperas_bus > 0 {
        filas.insert(
            6,
            (
                "esperas por el bus",
                ma.esperas_bus as f64,
                mb.esperas_bus as f64,
            ),
        );
    }
    let mut s = format!(
        "{:<28} {:>16} {:>16} {:>12}\n",
        "metrica", a.nombre_maquina, b.nombre_maquina, "diferencia"
    );
    for (nombre, x, y) in filas {
        let dif = if x == 0.0 {
            "-".to_string()
        } else {
            format!("{:+.1} %", (y - x) / x * 100.0)
        };
        let cifra = |v: f64| {
            if v.fract() == 0.0 {
                format!("{v:.0}")
            } else {
                format!("{v:.3}")
            }
        };
        s.push_str(&format!(
            "{nombre:<28} {:>16} {:>16} {dif:>12}\n",
            cifra(x),
            cifra(y)
        ));
    }
    s
}
