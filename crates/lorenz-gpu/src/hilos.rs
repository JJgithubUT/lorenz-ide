//! Ejecucion de kernels por hilos agrupados en warps (RF-GPU-013 a RF-GPU-016, RF-GPU-019).
//!
//! **Modelo.** Los hilos se reparten en grupos; los de un grupo comparten memoria y pueden
//! esperarse en una barrera. Dentro de un grupo se agrupan en warps de `tamano_warp` hilos que
//! avanzan en lockstep: en cada emision el warp ejecuta *una* instruccion para todos sus hilos
//! activos. Cada hilo lleva su propio contador de programa; el warp emite la instruccion del menor
//! contador entre los hilos vivos, y activa solo los que estan en ese mismo punto. Cuando un salto
//! separa a los hilos, el warp corre primero un camino y despues el otro (*divergencia*); en
//! cuanto los contadores vuelven a coincidir, los hilos se reagrupan solos (D-102).
//!
//! Cada unidad de ejecucion emite como mucho una instruccion de warp por ciclo y corre un grupo
//! a la vez. Un acceso a memoria deja al warp esperando su latencia, tiempo que otros warps del
//! mismo grupo pueden aprovechar.

use crate::config::ConfigHilos;
use crate::error::ErrorGpu;
use crate::kernel::*;

/// Una instruccion de warp emitida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emision {
    pub unidad: usize,
    pub ciclo: u64,
    pub grupo: usize,
    pub warp: usize,
    pub pc: usize,
    /// Un bit por hilo del warp, el bit 0 es el hilo 0; 1 si participo.
    pub mascara: u64,
    /// Si en este momento los hilos vivos del warp estaban en puntos distintos.
    pub divergente: bool,
    pub texto: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricasHilos {
    /// Ciclos hasta que termina la ultima unidad.
    pub ciclos: u64,
    /// Instrucciones de warp emitidas.
    pub emisiones: u64,
    /// Emisiones hechas con el warp dividido.
    pub emisiones_divergentes: u64,
    /// Suma de hilos activos en todas las emisiones.
    pub carriles_activos: u64,
    /// Ciclos en que una unidad no tuvo ningun warp listo.
    pub ciclos_ociosos: u64,
    pub accesos_globales: u64,
    pub accesos_compartidos: u64,
    pub barreras: u64,
    pub hilos: u64,
    pub grupos: u64,
}

impl MetricasHilos {
    /// Fraccion de los carriles del warp que trabajaron en promedio.
    pub fn utilizacion_carriles(&self, tamano_warp: usize) -> f64 {
        if self.emisiones == 0 {
            0.0
        } else {
            self.carriles_activos as f64 / (self.emisiones * tamano_warp as u64) as f64
        }
    }
}

#[derive(Debug, Clone)]
struct Hilo {
    pc: usize,
    regs: [i64; REGISTROS],
    vivo: bool,
    en_barrera: bool,
}

#[derive(Debug, Clone)]
struct Warp {
    hilos: Vec<Hilo>,
    listo_en: u64,
}

impl Warp {
    fn vivo(&self) -> bool {
        self.hilos.iter().any(|h| h.vivo)
    }

    /// Todos los hilos vivos del warp esperan en la barrera.
    fn en_barrera(&self) -> bool {
        self.vivo() && self.hilos.iter().filter(|h| h.vivo).all(|h| h.en_barrera)
    }
}

#[derive(Debug, Clone)]
struct GrupoEnCurso {
    indice: usize,
    warps: Vec<Warp>,
    compartida: Vec<i64>,
    ultimo: usize,
}

#[derive(Debug, Clone)]
struct Unidad {
    t: u64,
    grupo: Option<GrupoEnCurso>,
    fin: bool,
}

#[derive(Debug, Clone)]
pub struct EjecucionHilos {
    config: ConfigHilos,
    kernel: Kernel,
    grupos: usize,
    hilos_por_grupo: usize,
    global: Vec<i64>,
    unidades: Vec<Unidad>,
    siguiente_grupo: usize,
    metricas: MetricasHilos,
    /// Segun el kernel, cuantas palabras de memoria compartida tiene cada grupo.
    palabras_compartidas: usize,
}

fn fallo(m: String) -> ErrorGpu {
    ErrorGpu::Config(m)
}

impl EjecucionHilos {
    /// Prepara el lanzamiento de `grupos` grupos de `hilos` hilos cada uno. La memoria global
    /// parte de ceros mas los `.dato` del kernel.
    pub fn nueva(
        config: ConfigHilos,
        kernel: Kernel,
        grupos: usize,
        hilos: usize,
    ) -> Result<Self, ErrorGpu> {
        if grupos == 0 || hilos == 0 {
            return Err(fallo(
                "el lanzamiento necesita al menos un grupo y un hilo".into(),
            ));
        }
        if grupos.saturating_mul(hilos) > 1 << 22 {
            return Err(fallo(format!(
                "{} hilos son demasiados para simular",
                grupos * hilos
            )));
        }
        let tam_global = kernel.global.unwrap_or(config.memoria_global);
        if tam_global > crate::config::MEMORIA_MAXIMA {
            return Err(fallo(format!("`.global {tam_global}` es demasiado grande")));
        }
        let mut global = vec![0i64; tam_global];
        for (a, vs) in &kernel.datos {
            if a + vs.len() > global.len() {
                return Err(fallo(format!(
                    "`.dato {a}` escribe fuera de la memoria global de {} palabras",
                    global.len()
                )));
            }
            global[*a..*a + vs.len()].copy_from_slice(vs);
        }
        let palabras_compartidas = kernel.compartida.unwrap_or(config.memoria_compartida);
        let unidades = (0..config.unidades)
            .map(|_| Unidad {
                t: 0,
                grupo: None,
                fin: false,
            })
            .collect();
        Ok(Self {
            config,
            kernel,
            grupos,
            hilos_por_grupo: hilos,
            global,
            unidades,
            siguiente_grupo: 0,
            metricas: MetricasHilos {
                hilos: (grupos * hilos) as u64,
                grupos: grupos as u64,
                ..Default::default()
            },
            palabras_compartidas,
        })
    }

    pub fn config(&self) -> &ConfigHilos {
        &self.config
    }

    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    pub fn global(&self) -> &[i64] {
        &self.global
    }

    pub fn metricas(&self) -> MetricasHilos {
        self.metricas
    }

    /// Si ya no queda nada por ejecutar.
    pub fn terminada(&self) -> bool {
        self.siguiente_grupo >= self.grupos && self.unidades.iter().all(|u| u.grupo.is_none())
    }

    fn nuevo_grupo(&self, indice: usize) -> GrupoEnCurso {
        let w = self.config.tamano_warp;
        let warps = (0..self.hilos_por_grupo.div_ceil(w))
            .map(|i| {
                let n = (self.hilos_por_grupo - i * w).min(w);
                Warp {
                    hilos: (0..n)
                        .map(|_| Hilo {
                            pc: 0,
                            regs: [0; REGISTROS],
                            vivo: true,
                            en_barrera: false,
                        })
                        .collect(),
                    listo_en: 0,
                }
            })
            .collect();
        GrupoEnCurso {
            indice,
            warps,
            compartida: vec![0; self.palabras_compartidas],
            ultimo: usize::MAX,
        }
    }

    /// Valores de los registros de un hilo de un grupo que esta en curso, para depurar.
    pub fn registros_de(
        &self,
        grupo: usize,
        warp: usize,
        carril: usize,
    ) -> Option<([i64; REGISTROS], usize, bool)> {
        self.unidades.iter().find_map(|u| {
            let g = u.grupo.as_ref().filter(|g| g.indice == grupo)?;
            let h = g.warps.get(warp)?.hilos.get(carril)?;
            Some((h.regs, h.pc, h.vivo))
        })
    }

    /// Memoria compartida del grupo en curso `grupo`, si existe.
    pub fn compartida_de(&self, grupo: usize) -> Option<&[i64]> {
        self.unidades
            .iter()
            .find_map(|u| u.grupo.as_ref().filter(|g| g.indice == grupo))
            .map(|g| g.compartida.as_slice())
    }

    /// Grupos que estan corriendo ahora: `(unidad, grupo, warps)`.
    pub fn en_curso(&self) -> Vec<(usize, usize, usize)> {
        self.unidades
            .iter()
            .enumerate()
            .filter_map(|(i, u)| u.grupo.as_ref().map(|g| (i, g.indice, g.warps.len())))
            .collect()
    }

    /// Emite la siguiente instruccion de warp. Devuelve `None` cuando todo termino.
    pub fn paso(&mut self) -> Result<Option<Emision>, ErrorGpu> {
        loop {
            // La unidad que va mas atrasada en el tiempo es la que actua; a igualdad, la primera.
            let Some(u) = (0..self.unidades.len())
                .filter(|i| !self.unidades[*i].fin)
                .min_by_key(|i| (self.unidades[*i].t, *i))
            else {
                self.metricas.ciclos = self.unidades.iter().map(|u| u.t).max().unwrap_or(0);
                return Ok(None);
            };
            if self.unidades[u].grupo.is_none() {
                if self.siguiente_grupo >= self.grupos {
                    self.unidades[u].fin = true;
                    continue;
                }
                let g = self.nuevo_grupo(self.siguiente_grupo);
                self.siguiente_grupo += 1;
                self.unidades[u].grupo = Some(g);
            }
            if let Some(e) = self.emitir(u)? {
                self.metricas.ciclos = self.unidades.iter().map(|u| u.t).max().unwrap_or(0);
                return Ok(Some(e));
            }
        }
    }

    /// Ejecuta hasta el final y devuelve las emisiones si `registrar` (hasta `tope`). Un kernel que
    /// no termina se corta tras `limite` emisiones.
    pub fn ejecutar(
        &mut self,
        limite: u64,
        registrar: bool,
        tope: usize,
    ) -> Result<Vec<Emision>, ErrorGpu> {
        let mut v = Vec::new();
        while let Some(e) = self.paso()? {
            if registrar && v.len() < tope {
                v.push(e);
            }
            if self.metricas.emisiones >= limite && !self.terminada() {
                return Err(ErrorGpu::LimiteDeEmisiones { limite });
            }
        }
        Ok(v)
    }

    /// Intenta emitir una instruccion en la unidad `u`; `None` si solo avanzo el tiempo o termino
    /// el grupo.
    fn emitir(&mut self, u: usize) -> Result<Option<Emision>, ErrorGpu> {
        let t = self.unidades[u].t;
        let mut grupo = self.unidades[u]
            .grupo
            .take()
            .expect("la unidad tiene grupo");
        // Liberar la barrera si todos los warps vivos ya llegaron.
        let vivos: Vec<usize> = (0..grupo.warps.len())
            .filter(|i| grupo.warps[*i].vivo())
            .collect();
        if vivos.is_empty() {
            return Ok(None); // grupo terminado: la unidad queda libre
        }
        if vivos.iter().all(|i| grupo.warps[*i].en_barrera()) {
            for i in &vivos {
                for h in grupo.warps[*i].hilos.iter_mut() {
                    h.en_barrera = false;
                }
                grupo.warps[*i].listo_en = grupo.warps[*i].listo_en.max(t + 1);
            }
            self.metricas.barreras += 1;
        }
        // Elegir un warp listo, por turnos.
        let n = grupo.warps.len();
        let listo = (1..=n)
            .map(|k| (grupo.ultimo.wrapping_add(k)) % n)
            .find(|i| {
                let w = &grupo.warps[*i];
                w.vivo() && !w.en_barrera() && w.listo_en <= t
            });
        let Some(w) = listo else {
            // Nadie listo: el tiempo salta hasta el primero que lo estara.
            let proximo = grupo
                .warps
                .iter()
                .filter(|w| w.vivo() && !w.en_barrera())
                .map(|w| w.listo_en)
                .min();
            let nuevo = proximo.map_or(t + 1, |p| p.max(t + 1));
            self.metricas.ciclos_ociosos += nuevo - t;
            self.unidades[u].t = nuevo;
            self.unidades[u].grupo = Some(grupo);
            return Ok(None);
        };
        grupo.ultimo = w;
        let resultado = self.ejecutar_warp(u, t, &mut grupo, w);
        self.unidades[u].grupo = Some(grupo);
        let (e, latencia) = resultado?;
        self.unidades[u].t = t + 1;
        self.unidades[u].grupo.as_mut().expect("grupo").warps[w].listo_en = t + latencia.max(1);
        Ok(Some(e))
    }

    fn ejecutar_warp(
        &mut self,
        unidad: usize,
        t: u64,
        grupo: &mut GrupoEnCurso,
        w: usize,
    ) -> Result<(Emision, u64), ErrorGpu> {
        let k = &self.config;
        let warp = &mut grupo.warps[w];
        let pc = warp
            .hilos
            .iter()
            .filter(|h| h.vivo && !h.en_barrera)
            .map(|h| h.pc)
            .min()
            .expect("un warp listo tiene hilos vivos");
        let divergente = {
            let mut pcs = warp
                .hilos
                .iter()
                .filter(|h| h.vivo && !h.en_barrera)
                .map(|h| h.pc);
            let primero = pcs.next();
            pcs.any(|p| Some(p) != primero)
        };
        let instr = *self.kernel.instrucciones.get(pc).ok_or_else(|| {
            fallo(format!(
                "un hilo salio del kernel: el contador {pc} no tiene instruccion"
            ))
        })?;
        let mut mascara = 0u64;
        let mut latencia = 1u64;
        for (carril, h) in warp.hilos.iter_mut().enumerate() {
            if !h.vivo || h.en_barrera || h.pc != pc {
                continue;
            }
            mascara |= 1 << carril;
            let global_id = grupo.indice * self.hilos_por_grupo + w * k.tamano_warp + carril;
            let tid = w * k.tamano_warp + carril;
            let en = |m: String| {
                fallo(format!(
                    "grupo {}, warp {w}, hilo {carril} (linea {}): {m}",
                    grupo.indice, self.kernel.lineas[pc]
                ))
            };
            let valor = |f: Fuente, regs: &[i64; REGISTROS]| match f {
                Fuente::Reg(r) => regs[r],
                Fuente::Imm(i) => i,
            };
            h.pc += 1;
            match instr {
                Instr::Li(d, v) => h.regs[d] = v,
                Instr::Mov(d, s) => h.regs[d] = h.regs[s],
                Instr::Esp(d, e) => {
                    h.regs[d] = match e {
                        Especial::Tid => tid,
                        Especial::Gid => global_id,
                        Especial::Bid => grupo.indice,
                        Especial::Bdim => self.hilos_por_grupo,
                    } as i64
                }
                Instr::Alu(op, d, a, b) => {
                    let (x, y) = (h.regs[a], valor(b, &h.regs));
                    h.regs[d] = match op {
                        OpAlu::Add => x.wrapping_add(y),
                        OpAlu::Sub => x.wrapping_sub(y),
                        OpAlu::Mul => x.wrapping_mul(y),
                        OpAlu::Div | OpAlu::Rem if y == 0 => {
                            return Err(en("division por cero".into()))
                        }
                        OpAlu::Div => x.wrapping_div(y),
                        OpAlu::Rem => x.wrapping_rem(y),
                        OpAlu::And => x & y,
                        OpAlu::Or => x | y,
                        OpAlu::Xor => x ^ y,
                        OpAlu::Shl => x.wrapping_shl(y.clamp(0, 63) as u32),
                        OpAlu::Shr => x.wrapping_shr(y.clamp(0, 63) as u32),
                        OpAlu::Slt => i64::from(x < y),
                        OpAlu::Seq => i64::from(x == y),
                    };
                    if matches!(op, OpAlu::Mul | OpAlu::Div | OpAlu::Rem) {
                        latencia = latencia.max(k.ciclos_multiplicar);
                    }
                }
                Instr::Ld(d, r, off, comp) => {
                    let a = h.regs[r].wrapping_add(off);
                    let (mem, nombre) = if comp {
                        (&grupo.compartida, "compartida")
                    } else {
                        (&self.global, "global")
                    };
                    let v = *usize::try_from(a)
                        .ok()
                        .and_then(|a| mem.get(a))
                        .ok_or_else(|| {
                            en(format!(
                                "lee la direccion {a} fuera de la memoria {nombre} de {} palabras",
                                mem.len()
                            ))
                        })?;
                    h.regs[d] = v;
                    latencia = latencia.max(if comp {
                        k.latencia_compartida
                    } else {
                        k.latencia_global
                    });
                }
                Instr::St(r, off, s, comp) => {
                    let a = h.regs[r].wrapping_add(off);
                    let v = h.regs[s];
                    let (mem, nombre) = if comp {
                        (&mut grupo.compartida, "compartida")
                    } else {
                        (&mut self.global, "global")
                    };
                    let largo = mem.len();
                    let celda = usize::try_from(a)
                        .ok()
                        .and_then(|a| mem.get_mut(a))
                        .ok_or_else(|| en(format!("escribe la direccion {a} fuera de la memoria {nombre} de {largo} palabras")))?;
                    *celda = v;
                    latencia = latencia.max(if comp {
                        k.latencia_compartida
                    } else {
                        k.latencia_global
                    });
                }
                Instr::Bra(d) => h.pc = d,
                Instr::Bz(r, d) => {
                    if h.regs[r] == 0 {
                        h.pc = d
                    }
                }
                Instr::Bnz(r, d) => {
                    if h.regs[r] != 0 {
                        h.pc = d
                    }
                }
                Instr::Bar => h.en_barrera = true,
                Instr::Exit => h.vivo = false,
            }
        }
        let activos = u64::from(mascara.count_ones());
        self.metricas.emisiones += 1;
        self.metricas.carriles_activos += activos;
        if divergente {
            self.metricas.emisiones_divergentes += 1;
        }
        if let Instr::Ld(.., c) | Instr::St(.., c) = instr {
            if c {
                self.metricas.accesos_compartidos += 1;
            } else {
                self.metricas.accesos_globales += 1;
            }
        }
        Ok((
            Emision {
                unidad,
                ciclo: t,
                grupo: grupo.indice,
                warp: w,
                pc,
                mascara,
                divergente,
                texto: self.kernel.textos[pc].clone(),
            },
            latencia,
        ))
    }
}
