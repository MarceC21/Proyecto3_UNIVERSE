// Ventana (raylib) con el sistema solar y la nave, dibujados con NUESTRO rasterizador.
// raylib solo abre la ventana, lee teclado/ratón y muestra el framebuffer + HUD.
use nave_raster::{
    camara::{Camara, Entrada},
    cielo::Cielo,
    dibujo::Ajustes,
    escena::Escena,
    math::{v3, V3},
    nave::{Mundo, Nave},
    raster::Framebuffer,
    render::dibujar_con_nave as dibujar,
};
use nave_raster::seguimiento::Seguimiento;
use nave_raster::mision_marte::{FaseMarte, MisionMarte};
use nave_raster::terminal_marte::TerminalMarte;
use nave_raster::progreso::{Galaxia, Progreso};
use nave_raster::salto::Salto;
use nave_raster::mapa::Mapa;
use raylib::prelude::*;
use nave_raster::aventura::{Aventura, Evento as EventoInterior};

fn completar_marte(mision: &mut MisionMarte, progreso: &mut Progreso) {
    if mision.maquina.aceptada() {
        mision.marcar_completada();
        progreso.completar_marte();
    }
}

fn resolucion(calidad: usize) -> (usize, usize) {
    match calidad { 1 => (960, 600), 2 => (720, 450), _ => (480, 300) }
}

fn cargar_sistema(galaxia: Galaxia) -> (Escena, Cielo, V3, V3) {
    match galaxia {
        Galaxia::Solar => {
            let escena = Escena::nueva();
            let (pos,obj) = nave_raster::guia::inicio_solar(&escena);
            (escena,Cielo::nuevo(),pos,obj)
        }
        Galaxia::Gargantua => (Escena::gargantua(),Cielo::gargantua(),nave_raster::gargantua::INICIO,V3::ZERO),
        Galaxia::TauCeti => (Escena::tau_ceti(),Cielo::tau_ceti(),nave_raster::tau_ceti::INICIO,V3::ZERO),
        Galaxia::Eridani => unreachable!("Sistema todavia no disponible"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut ruta = None;
    let mut galaxia_inicial = Galaxia::Solar;
    let mut iniciar_miller = false;
    let mut iniciar_adrian = false;
    let mut iniciar_estacion = false;
    while let Some(arg) = args.next() {
        if arg == "--adrian" || arg == "--adrien" {
            iniciar_adrian = true;
        } else if arg == "--estacion" {
            iniciar_estacion = true;
        } else if arg == "--miller" {
            iniciar_miller = true;
            galaxia_inicial = Galaxia::Gargantua;
        } else if arg == "--galaxia" {
            galaxia_inicial = match args.next().as_deref() {
                Some("gargantua") => Galaxia::Gargantua,
                Some("solar") => Galaxia::Solar,
                Some("tau-ceti") | Some("tau_ceti") => Galaxia::TauCeti,
                _ => { eprintln!("Usa --galaxia solar, gargantua o tau-ceti"); std::process::exit(1); }
            };
        } else if ruta.is_none() { ruta = Some(arg); }
        else { eprintln!("Argumento no reconocido: {arg}"); std::process::exit(1); }
    }
    if iniciar_miller { galaxia_inicial = Galaxia::Gargantua; }
    if iniciar_adrian { galaxia_inicial = Galaxia::TauCeti; iniciar_miller = false; }
    if iniciar_estacion { galaxia_inicial = Galaxia::Solar; iniciar_miller = false; iniciar_adrian = false; }
    let mut nave = match ruta.as_deref() {
        Some(ruta) => Nave::cargar(ruta),
        None => Nave::predeterminada(),
    }.unwrap_or_else(|error| { eprintln!("No se pudo cargar la nave: {error}"); std::process::exit(1) });
    let (mut window, thread) = raylib::init().size(1280, 800).title("Sistema solar - rasterizador por software").build();
    window.set_target_fps(60);
    // Escape vuelve al vuelo desde la terminal; en vuelo cierra el programa.
    window.set_exit_key(None);
    window.disable_cursor();

    println!("Generando texturas y {}...",galaxia_inicial.nombre());
    let (mut escena, mut cielo, inicio, objetivo_inicial) = cargar_sistema(galaxia_inicial);
    nave.establecer_reaparicion(inicio);
    nave.teletransportar(inicio,objetivo_inicial);

    let mut calidad = 1usize;
    let (mut rw, mut rh) = resolucion(calidad);
    let mut fb = Framebuffer::new(rw, rh);
    let mut cam = Camara::nueva(rw as f32, rh as f32);
    let mut seguimiento = Seguimiento::nuevo(nave.fwd, nave.up);
    let mut vistas = escena.vistas();

    let mut textura: Option<Texture2D> = None;
    let mut aj = Ajustes::default();
    let mut tiempo = 0.0f32;
    let mut mouse = true;
    let mut camara_libre = false;
    let mut mision_marte = MisionMarte::default();
    let mut terminal_marte = TerminalMarte::default();
    // Cada ejecución es una partida nueva; el avance vive solo en memoria.
    let mut progreso = Progreso::default();
    if galaxia_inicial != Galaxia::Solar {
        // Acceso de inspección; la partida normal conserva el desbloqueo de Marte.
        progreso.completar_estacion();
        progreso.completar_marte();
        progreso.viajar(Galaxia::Gargantua);
        if galaxia_inicial == Galaxia::TauCeti {
            progreso.completar(Galaxia::Gargantua);
            progreso.viajar(Galaxia::TauCeti);
        }
        mision_marte.marcar_completada();
    }
    let mut salto = Salto::default();
    let mut mapa = Mapa::default();
    let mut mapa_abierto = false;
    let mut miller = nave_raster::interfaz_miller::SuperficieMiller::default();
    let mut miller_armado = true;
    if iniciar_miller { miller.entrar(); }
    let mut aventura: Option<Aventura> = None;
    let mut visor_interior = nave_raster::interfaz_interior::VisorInterior::default();
    let mut blip_armada = true;
    let mut operacion_adrian = nave_raster::interfaz_adrian::OperacionAdrian::default();
    let mut adrian_armado = true;
    if iniciar_adrian {
        progreso.completar_encuentro_blip();
        operacion_adrian.entrar(&nave);
    }
    if iniciar_estacion {
        let est = escena.estacion.as_ref().expect("falta estacion");
        let pos = est.pos + v3(0.0, 3.0, 10.0);
        nave.teletransportar(pos, est.pos);
        let mut a = Aventura::nueva();
        a.iniciar_atraque(nave.pos, nave.fwd, est.pos, est.radio, est.rot);
        aventura = Some(a);
    }

    while !window.window_should_close() {
        let dt = window.get_frame_time().clamp(0.0005, 0.1);
        tiempo += dt;

        if let Some(a) = aventura.as_mut().filter(|a| a.activa()) {
            let evento = visor_interior.cuadro(&mut window, &thread, a, &mut nave, &escena, &cielo, &aj, dt);
            if a.encuentro_blip_realizado() { progreso.completar_encuentro_blip(); }
            if evento == EventoInterior::Desacoplar {
                if a.completada() { progreso.completar_estacion(); }
                let (pos, objetivo) = a.pose_desacople();
                nave.teletransportar(pos, objetivo);
                nave.modo_local = false;
                seguimiento.reiniciar(nave.fwd, nave.up);
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            } else if evento == EventoInterior::VolverAlVuelo {
                seguimiento.reiniciar(nave.fwd,nave.up);
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }

        if miller.activa {
            let salir = miller.cuadro(&mut window, &thread, &nave);
            if miller.exitosa() { progreso.completar(Galaxia::Gargantua); }
            if salir {
                let planeta = escena.cuerpos.iter().find(|c| c.nombre == "Miller").expect("falta Miller");
                // Despegar por el lado exterior evita caer hacia Gargantua.
                let radial = planeta.pos.norm();
                let pos = planeta.pos + radial * (planeta.radio + 12.0);
                nave.teletransportar(pos, pos + radial);
                nave.modo_local = false;
                seguimiento.reiniciar(nave.fwd, nave.up);
                miller_armado = false;
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }

        if operacion_adrian.activa {
            let salir = operacion_adrian.cuadro(&mut window,&thread,&nave);
            if operacion_adrian.exitosa() { progreso.completar(Galaxia::TauCeti); }
            if salir {
                adrian_armado = false;
                seguimiento.reiniciar(nave.fwd,nave.up);
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }

        // La misión conserva su progreso entre visitas y consume su propia
        // entrada: al seleccionar botones nunca se mueve ni daña la nave.
        if mision_marte.fase != FaseMarte::Espacio {
            mision_marte.actualizar(dt);
            completar_marte(&mut mision_marte, &mut progreso);
            if terminal_marte.cuadro(&mut window, &thread, &mut mision_marte, &nave, tiempo, textura.as_ref()) {
                mision_marte.despegar();
                completar_marte(&mut mision_marte, &mut progreso);
                let marte = escena.cuerpos.iter().find(|c| c.nombre == "Marte").expect("falta Marte");
                let radial = (escena.pos_sol() - marte.pos).norm();
                let pos = marte.pos + radial * (marte.radio + 12.0);
                nave.teletransportar(pos, pos + radial);
                nave.modo_local = false;
                seguimiento.reiniciar(nave.fwd, nave.up);
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }
        // Abrir y dibujar en el mismo cuadro: EndDrawing actualiza el teclado.
        let mapa_recien_abierto = !mapa_abierto && window.is_key_pressed(KeyboardKey::KEY_M);
        if mapa_recien_abierto {
            mapa_abierto = true;
            window.enable_cursor();
        }
        if mapa_abierto {
            salto.actualizar(dt);
            let cerrar = mapa.cuadro(&mut window, &thread, &mut progreso, &salto, tiempo, textura.as_ref(), mapa_recien_abierto);
            if !cerrar && salto.listo() {
                if let Some(solicitud) = salto.solicitud().filter(|s| s.destino.implementada()) {
                    if progreso.viajar(solicitud.destino) {
                        let (nueva_escena,nuevo_cielo,inicio,objetivo) = cargar_sistema(solicitud.destino);
                        escena = nueva_escena;
                        cielo = nuevo_cielo;
                        vistas = escena.vistas();
                        nave.establecer_reaparicion(inicio);
                        nave.reaparecer();
                        nave.teletransportar(inicio,objetivo);
                        nave.modo_local = false;
                        seguimiento.reiniciar(nave.fwd, nave.up);
                        camara_libre = false;
                        salto.volver();
                        mapa_abierto = false;
                        if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
                    }
                }
            }
            if cerrar {
                mapa_abierto = false;
                if salto.solicitud().is_some() {
                    // Cancelar la sincronización devuelve la nave a una posición
                    // segura dentro del sistema que se estaba explorando.
                    salto.volver();
                    let radial = v3(nave.pos.x, 0.0, nave.pos.z).norm();
                    let pos = radial * (escena.radio_limite - 22.0) + v3(0.0, nave.pos.y, 0.0);
                    nave.teletransportar(pos, pos - radial);
                    seguimiento.reiniciar(nave.fwd, nave.up);
                }
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }
        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) { break; }

        // ----- Interruptores -----
        if window.is_key_pressed(KeyboardKey::KEY_F1) { calidad = 1; }
        if window.is_key_pressed(KeyboardKey::KEY_F2) { calidad = 2; }
        if window.is_key_pressed(KeyboardKey::KEY_F3) { calidad = 3; }
        if window.is_key_pressed(KeyboardKey::KEY_C) { aj.culling = !aj.culling; }
        if window.is_key_pressed(KeyboardKey::KEY_L) { aj.lod = !aj.lod; }
        if window.is_key_pressed(KeyboardKey::KEY_O) { aj.orbitas = !aj.orbitas; }
        if window.is_key_pressed(KeyboardKey::KEY_B) { aj.limite = !aj.limite; }
        if window.is_key_pressed(KeyboardKey::KEY_F) { nave.modo_local = !nave.modo_local; }
        if window.is_key_pressed(KeyboardKey::KEY_R) {
            nave.reaparecer();
            if escena.galaxia == Galaxia::Solar {
                let (_,obj) = nave_raster::guia::inicio_solar(&escena);
                nave.teletransportar(nave.pos,obj);
            }
            seguimiento.reiniciar(nave.fwd, nave.up);
        }
        if window.is_key_pressed(KeyboardKey::KEY_TAB) {
            mouse = !mouse;
            if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
        }
        if window.is_key_pressed(KeyboardKey::KEY_V) {
            camara_libre = !camara_libre;
            cam.vel = V3::ZERO;
            if camara_libre {
                // La cámara libre arranca donde estaba la de persecución.
                let (f, _, _) = cam.base();
                cam.pitch = f.y.clamp(-1.0, 1.0).asin();
                cam.yaw = f.x.atan2(-f.z);
                cam.base_ext = None;
                cam.fov = 62f32.to_radians();
            }
        }
        let teclas = if escena.galaxia == Galaxia::Solar {
            vec![(KeyboardKey::KEY_ZERO, "general"), (KeyboardKey::KEY_ONE, "sol"),
                (KeyboardKey::KEY_TWO, "mercurio"), (KeyboardKey::KEY_THREE, "venus"),
                (KeyboardKey::KEY_FOUR, "tierra"), (KeyboardKey::KEY_FIVE, "marte"),
                (KeyboardKey::KEY_EIGHT, "jupiter"), (KeyboardKey::KEY_NINE, "saturno"),
                (KeyboardKey::KEY_U, "urano"), (KeyboardKey::KEY_N, "neptuno"),
                (KeyboardKey::KEY_SIX, "cinturon"), (KeyboardKey::KEY_SEVEN, "limite"),
                (KeyboardKey::KEY_T, "estacion")]
        } else if escena.galaxia == Galaxia::Gargantua {
            vec![(KeyboardKey::KEY_ZERO, "general"), (KeyboardKey::KEY_ONE, "gargantua"),
                (KeyboardKey::KEY_TWO, "miller"), (KeyboardKey::KEY_THREE, "mann"),
                (KeyboardKey::KEY_FOUR, "edmunds"), (KeyboardKey::KEY_FIVE, "thalos"),
                (KeyboardKey::KEY_SIX, "nyx"), (KeyboardKey::KEY_SEVEN, "aurora"),
                (KeyboardKey::KEY_EIGHT, "limite")]
        } else {
            vec![(KeyboardKey::KEY_ZERO,"general"),(KeyboardKey::KEY_ONE,"tau ceti"),
                (KeyboardKey::KEY_TWO,"adrien"),(KeyboardKey::KEY_THREE,"gigante"),
                (KeyboardKey::KEY_FOUR,"ceti nova"),(KeyboardKey::KEY_FIVE,"oceanus"),
                (KeyboardKey::KEY_SIX,"saturn"),(KeyboardKey::KEY_SEVEN,"ceti nova ii"),
                (KeyboardKey::KEY_EIGHT,"veridia"),(KeyboardKey::KEY_NINE,"aethel"),
                (KeyboardKey::KEY_U,"cinturon"),(KeyboardKey::KEY_N,"limite"),
                (KeyboardKey::KEY_T,"nave_cafe")]
        };
        for (tecla, nombre) in teclas {
            if window.is_key_pressed(tecla) {
                if let Some((_, pos, objetivo)) = vistas.iter().find(|v| v.0 == nombre) {
                    if camara_libre { cam.pos = *pos; cam.vel = V3::ZERO; cam.mirar_a(*objetivo); }
                    else { nave.teletransportar(*pos, *objetivo); seguimiento.reiniciar(nave.fwd, nave.up); }
                }
            }
        }

        let (nw, nh) = resolucion(calidad);
        if (nw, nh) != (rw, rh) {
            rw = nw;
            rh = nh;
            fb.redimensionar(rw, rh);
            cam.ajustar_resolucion(rw as f32, rh as f32);
            textura = None;
        }

        // ----- Entrada -----
        let eje = |mas: KeyboardKey, menos: KeyboardKey, w: &RaylibHandle| {
            (w.is_key_down(mas) as i32 - w.is_key_down(menos) as i32) as f32
        };
        let raton = if mouse { window.get_mouse_delta() } else { Vector2::zero() };
        let orbitando = !camara_libre && window.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT);

        escena.actualizar(dt);
        let cuerpos = escena.esferas();
        if camara_libre {
            let mut giro = raton;
            giro.x += 420.0 * dt * eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT, &window);
            giro.y += 420.0 * dt * eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN, &window);
            let entrada = Entrada {
                adelante: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S, &window),
                lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A, &window),
                vertical: eje(KeyboardKey::KEY_SPACE, KeyboardKey::KEY_LEFT_CONTROL, &window),
                giro_x: giro.x,
                giro_y: giro.y,
                turbo: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
            };
            cam.actualizar(&entrada, dt, &cuerpos, escena.radio_limite, escena.alto_limite);
        } else {
            let control = nave_raster::entrada_nave::leer(&window,
                if orbitando { (0.0,0.0) } else { (raton.x,raton.y) });
            let mundo = Mundo {
                cuerpos: &cuerpos,
                asteroides: escena.colisionadores_asteroides(),
                radio_limite: escena.radio_limite,
                alto_limite: escena.alto_limite,
                cinturon: escena.cinturon,
                gravedad: escena.agujero_negro.as_ref().map(|a| a.campo),
            };
            nave.actualizar(&control, dt, &mundo);
            seguimiento.actualizar(&mut cam, nave.pos, nave.fwd, nave.up, nave.velocidad(), dt,
                if orbitando { Some((raton.x, raton.y)) } else { None }, &cuerpos);
        }

        let altura_marte = escena.cuerpos.iter().find(|c| c.nombre == "Marte")
            .map(|c| (nave.pos - c.pos).len() - c.radio).unwrap_or(f32::INFINITY);
        if window.is_key_pressed(KeyboardKey::KEY_H) && progreso.estacion_completada()
            && !camara_libre && nave.destruida <= 0.0 {
            aventura.get_or_insert_with(Aventura::nueva).entrar_cabina(nave.pos,nave.fwd,nave.up,true);
        }
        if window.is_key_pressed(KeyboardKey::KEY_E) && !camara_libre && nave.destruida <= 0.0 {
            if let Some(est) = &escena.estacion {
                if Aventura::puede_atracar(nave.pos,nave.velocidad(),est.pos,est.radio,true) {
                    let a = aventura.get_or_insert_with(Aventura::nueva);
                    a.iniciar_atraque(nave.pos,nave.fwd,est.pos,est.radio,est.rot);
                    nave.vel = V3::ZERO;
                }
            }
        }
        let mut en_vuelo = !camara_libre && nave.destruida <= 0.0 && !aventura.as_ref().is_some_and(|a| a.activa());
        if let Some(n) = &escena.nave_cafe {
            let altura = (nave.pos-n.pos).len()-n.radio;
            if altura > 10.0 { blip_armada = true; }
            if blip_armada && en_vuelo && progreso.estacion_completada()
                && (0.0..=6.0).contains(&altura) && nave.velocidad() <= 6.0 {
                aventura.get_or_insert_with(Aventura::nueva).iniciar_atraque_blip(nave.pos,nave.fwd,n.pos,n.radio,n.rot);
                blip_armada = false;
                nave.vel = V3::ZERO;
                nave.modo_local = false;
                en_vuelo = false;
            }
        }
        if mision_marte.observar(altura_marte, nave.velocidad(), en_vuelo && progreso.marte_disponible()) {
            nave.vel = V3::ZERO;
            window.enable_cursor();
        }
        let altura_miller = escena.cuerpos.iter().find(|c| c.nombre == "Miller")
            .map(|c| (nave.pos - c.pos).len() - c.radio).unwrap_or(f32::INFINITY);
        if altura_miller > 10.0 { miller_armado = true; }
        if escena.galaxia == Galaxia::Gargantua && miller_armado && !progreso.completada(Galaxia::Gargantua)
            && en_vuelo && (0.0..=6.5).contains(&altura_miller) && nave.velocidad() <= 6.0 {
            miller.entrar();
            miller_armado = false;
            nave.vel = V3::ZERO;
            window.disable_cursor();
        }
        let altura_adrian = escena.cuerpos.iter().find(|c| c.nombre == "Adrien")
            .map(|c| (nave.pos-c.pos).len()-c.radio).unwrap_or(f32::INFINITY);
        if altura_adrian > 10.0 { adrian_armado = true; }
        if escena.galaxia == Galaxia::TauCeti && en_vuelo && adrian_armado
            && progreso.adrien_disponible() && !progreso.completada(Galaxia::TauCeti)
            && (0.0..=6.5).contains(&altura_adrian) && nave.velocidad() <= 6.0 {
            operacion_adrian.entrar(&nave);
            adrian_armado = false;
            en_vuelo = false;
        }
        let distancia_borde_nave = escena.radio_limite - (nave.pos.x * nave.pos.x + nave.pos.z * nave.pos.z).sqrt();
        if salto.observar(&progreso, distancia_borde_nave,
            en_vuelo && mision_marte.fase == FaseMarte::Espacio && !miller.activa) {
            nave.vel = V3::ZERO;
            nave.modo_local = false;
            mapa_abierto = true;
            window.enable_cursor();
        }

        let st = dibujar(&mut fb, &escena, &cielo, &cam, tiempo, &aj, Some(&nave));

        if textura.is_none() {
            let img = Image::gen_image_color(rw as i32, rh as i32, Color::BLACK);
            let t = window.load_texture_from_image(&thread, &img).expect("no se pudo crear la textura");
            t.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            textura = Some(t);
        }
        let tex = textura.as_mut().unwrap();
        tex.update_texture(&fb.pixels).expect("buffer RGBA incompatible");

        let (sw, sh) = (window.get_screen_width() as f32, window.get_screen_height() as f32);
        let escala = (sw / rw as f32).min(sh / rh as f32);
        let (dw, dh) = (rw as f32 * escala, rh as f32 * escala);
        let (ox, oy) = ((sw - dw) * 0.5, (sh - dh) * 0.5);
        let referencia = if camara_libre { cam.pos } else { nave.pos };
        let (cercano, alt) = escena.cuerpo_cercano(referencia);
        let dist_lim = escena.radio_limite - (referencia.x * referencia.x + referencia.z * referencia.z).sqrt();
        let fps = window.get_fps();
        let punto_mira = cam.proyectar(nave.pos + nave.fwd * 40.0);

        let mut d = window.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(&*tex, Rectangle::new(0.0, 0.0, rw as f32, rh as f32),
            Rectangle::new(ox, oy, dw, dh), Vector2::zero(), 0.0, Color::WHITE);
        if !camara_libre {
            if let Some(objetivo) = nave_raster::guia::objetivo(&escena,&progreso,nave.pos) {
                nave_raster::guia::dibujar(&mut d,&cam,&objetivo,nave.pos,(ox,oy,dw,dh));
            }
        }

        // Etiquetas de los cuerpos proyectadas a pantalla.
        for c in &escena.cuerpos {
            if let Some((sx, sy, w)) = cam.proyectar(c.pos + v3(0.0, c.radio * 1.5, 0.0)) {
                if w < 400.0 {
                    let (x, y) = ((ox + sx * escala) as i32, (oy + sy * escala) as i32);
                    let t = format!("{} ({:.0})", c.nombre, (c.pos - referencia).len());
                    d.draw_text(&t, x - (t.len() as i32) * 3, y - 14, 16, Color::new(255, 255, 255, 230));
                }
            }
        }
        if let Some(estacion) = &escena.estacion {
            if let Some((sx, sy, profundidad)) = cam.proyectar(estacion.pos + v3(0.0, estacion.radio, 0.0)) {
                if profundidad < 160.0 {
                    let texto = format!("Estacion ({:.0})", (referencia - estacion.pos).len());
                    d.draw_text(&texto, (ox + sx * escala) as i32 - 50,
                        (oy + sy * escala) as i32 - 14, 16, Color::new(100, 240, 245, 230));
                }
            }
        }
        if let Some(n) = &escena.nave_cafe {
            if let Some((sx,sy,_)) = cam.proyectar(n.pos+v3(0.0,n.radio,0.0)) {
                d.draw_text(&format!("Blip-A ({:.0})",(referencia-n.pos).len()),
                    (ox+sx*escala) as i32-55,(oy+sy*escala) as i32-16,17,Color::new(255,200,117,255));
            }
            if !camara_libre && (nave.pos-n.pos).len()-n.radio < 15.0 {
                d.draw_text(if progreso.nave_cafe_visitada() { "Blip-A: encuentro completado / SIGUIENTE DESTINO: ADRIEN" }
                    else { "Blip-A: acercate a menos de 6 y baja a 6 para atracar" },
                    16,sh as i32-112,17,Color::new(255,200,117,255));
            }
        }
        // Mira: hacia dónde apunta la nariz.
        if let Some((sx, sy, _)) = punto_mira {
            let (x, y) = ((ox + sx * escala) as i32, (oy + sy * escala) as i32);
            let c = Color::new(120, 255, 240, 200);
            d.draw_rectangle(x - 10, y - 1, 7, 2, c);
            d.draw_rectangle(x + 4, y - 1, 7, 2, c);
            d.draw_rectangle(x - 1, y - 10, 2, 7, c);
            d.draw_rectangle(x - 1, y + 4, 2, 7, c);
        }

        d.draw_rectangle(8, 8, 760, 172, Color::new(0, 0, 0, 170));
        d.draw_text(&format!("{} FPS | velocidad {:.1} / {:.0} | cerca de {} ({:.1})",
            fps, nave.velocidad(), nave.velocidad_max, cercano, alt), 16, 14, 16, Color::WHITE);
        d.draw_text(&format!("Objetos {}/{}   Triangulos {} de {} (sin culling/LOD)",
            st.objetos_enviados, st.objetos_total, st.tris_enviados, st.tris_sin_optimizar), 16, 36, 16, Color::WHITE);
        d.draw_text(&format!("[C]ulling {}  [L]OD {}  [O]rbitas {}  [B]limite {}  {}",
            if aj.culling { "ON" } else { "off" }, if aj.lod { "ON" } else { "off" },
            if aj.orbitas { "ON" } else { "off" }, if aj.limite { "ON" } else { "off" },
            if camara_libre { "[V] CAMARA LIBRE" } else { "[V] camara libre" }), 16, 58, 16, Color::WHITE);
        d.draw_text("WASD mover  Espacio/Ctrl subir/bajar  Q/E alabeo  Raton/flechas girar  Shift turbo", 16, 84, 14, Color::LIGHTGRAY);
        d.draw_text("Clic derecho orbitar camara  F modo local  R reaparecer  V camara libre  TAB cursor", 16, 104, 14, Color::LIGHTGRAY);

        if escena.galaxia == Galaxia::Solar {
            d.draw_text("0 general  1 Sol 2 Mercurio 3 Venus 4 Tierra 5 Marte 6 cinturon 7 limite", 16, 124, 14, Color::LIGHTGRAY);
            d.draw_text("8 Jupiter  9 Saturno  U Urano  N Neptuno  T Estacion  M Mapa  F1-F3 calidad", 16, 144, 14, Color::LIGHTGRAY);
        } else if escena.galaxia == Galaxia::Gargantua {
            d.draw_text("GARGANTUA: 0 general  1 centro  2 Miller  3 Mann  4 Edmunds  5 Thalos", 16, 124, 14, Color::LIGHTGRAY);
            d.draw_text("6 Nyx  7 Aurora  8 limite  M Mapa  F1-F3 calidad", 16, 144, 14, Color::LIGHTGRAY);
        } else {
            d.draw_text("TAU CETI: 0 general  1 estrella  2 Adrien  3 Gigante  4 Ceti Nova  5 Oceanus",16,124,14,Color::LIGHTGRAY);
            d.draw_text("6 Saturn 7 Ceti Nova II 8 Veridia 9 Aethel T Blip-A U cinturon N limite M mapa",16,144,14,Color::LIGHTGRAY);
        }

        // Modo local (rodear un cuerpo).
        let objetivo_local = escena.cuerpos.iter().min_by(|a, b|
            (a.pos - nave.objetivo).len().total_cmp(&(b.pos - nave.objetivo).len()))
            .map(|c| c.nombre).unwrap_or(cercano);
        let local_txt = if camara_libre { None } else if nave.local_activo { Some(format!("MODO LOCAL: orbitando {objetivo_local}  (A/D/Espacio rodean, W/S acercan)")) }
            else if nave.modo_local { Some("MODO LOCAL activado: acércate a un cuerpo".to_string()) }
            else if alt < 25.0 { Some(format!("[F] modo local para rodear {cercano}")) } else { None };
        if let Some(t) = local_txt {
            d.draw_text(&t, 16, (sh as i32) - 60, 18, Color::new(120, 255, 240, 255));
        }
        if !camara_libre && altura_marte < 18.0 && !mision_marte.completada() {
            d.draw_text(if progreso.marte_disponible() { "MARTE: acercate a menos de 6.5 y reduce la velocidad a 6 para aterrizar" }
                else { "MARTE BLOQUEADO: visita la estacion, recoge provisiones e instala los sistemas primero [T]" },
                16, sh as i32 - 88, 17, Color::new(255, 190, 132, 255));
        }
        if escena.galaxia == Galaxia::Solar && !camara_libre {
            if !progreso.estacion_completada() {
                d.draw_text("PRIMERA MISION: visita la estacion junto a la Tierra [T]", 16, 188, 17, Color::new(111,232,239,255));
            }
            if let Some(est) = &escena.estacion {
                let cerca = (nave.pos-est.pos).len()-est.radio < 14.0;
                if cerca {
                    let texto = if Aventura::puede_atracar(nave.pos,nave.velocidad(),est.pos,est.radio,nave.destruida<=0.0) {
                        "[E] ATRACAR EN LA ESTACION - CABINA / TARS / PROVISIONES"
                    } else { "ESTACION: acercate y reduce la velocidad a 6 para atracar" };
                    d.draw_text(texto,16,sh as i32-112,17,Color::new(111,232,239,255));
                }
            }
        }
        if !camara_libre && altura_miller < 24.0 && !progreso.completada(Galaxia::Gargantua) {
            d.draw_text("MILLER: acercate a menos de 6.5 y reduce la velocidad a 6 para aterrizar",
                16, sh as i32 - 88, 17, Color::new(130, 230, 240, 255));
        }
        if !camara_libre && altura_adrian < 24.0 && !progreso.completada(Galaxia::TauCeti) {
            d.draw_text(if progreso.adrien_disponible() { "ADRIEN: acercate a menos de 6.5 y baja a 6 para iniciar OPERACION TAUMOEBA" }
                else { "ADRIEN: completa primero el encuentro del cristal en Blip-A" },
                16,sh as i32-88,17,Color::new(123,245,158,255));
        }
        if progreso.motor_activo() {
            d.draw_text("MOTOR LISTO / [M] MAPA INTERESTELAR", sw as i32 - 400, sh as i32 - 30,
                17, Color::new(131, 239, 163, 255));
        }
        if progreso.estacion_completada() && !camara_libre {
            d.draw_text("[H] ENTRAR A LA CABINA",sw as i32-315,sh as i32-56,17,Color::new(111,232,239,255));
        }

        if let Some(agujero) = &escena.agujero_negro {
            let intensidad = agujero.campo.intensidad(nave.pos);
            if !camara_libre && intensidad > 0.025 {
                let hacia_agujero = nave.local_activo && (nave.objetivo - agujero.campo.centro).len() < 0.01;
                let advertencia = if hacia_agujero { "ATRACCION: F LIBERA LA ORIENTACION / GIRA HACIA FUERA Y USA W + SHIFT" }
                    else if intensidad > 0.45 { "PELIGRO: ATRACCION FUERTE / ORIENTA HACIA FUERA Y USA W + SHIFT" }
                    else { "CAMPO GRAVITATORIO / LOS MOTORES DEBEN COMPENSAR LA ATRACCION" };
                d.draw_text(advertencia, 16, sh as i32 - 92, 17, Color::new(255, 186, 94, 255));
                d.draw_rectangle(16, sh as i32 - 69, 210, 5, Color::new(40, 30, 20, 220));
                d.draw_rectangle(16, sh as i32 - 69, (210.0 * intensidad) as i32, 5, Color::new(255, 166, 68, 255));
            }
        }

        // Integridad de la nave.
        let barra = (nave.integridad / 100.0).clamp(0.0, 1.0);
        d.draw_rectangle(16, sh as i32 - 34, 204, 18, Color::new(0, 0, 0, 170));
        let col = if barra > 0.5 { Color::new(90, 230, 140, 230) } else if barra > 0.25 { Color::new(250, 200, 70, 230) } else { Color::new(255, 70, 70, 230) };
        d.draw_rectangle(18, sh as i32 - 32, (200.0 * barra) as i32, 14, col);
        d.draw_text("INTEGRIDAD", 24, sh as i32 - 32, 12, Color::BLACK);

        if nave.golpe > 0.0 {
            d.draw_rectangle(0, 0, sw as i32, sh as i32, Color::new(255, 40, 40, (nave.golpe * 160.0) as u8));
        }
        if nave.destruida > 0.0 {
            d.draw_text("NAVE DESTRUIDA", (sw as i32 - 300) / 2, sh as i32 / 2 - 20, 40, Color::new(255, 80, 80, 255));
        }
        if dist_lim < 25.0 {
            let parpadeo = ((tiempo * 6.0).sin() * 0.5 + 0.5) * 120.0 + 135.0;
            let texto = if progreso.motor_activo() { "LIMITE: RUTA DE SALTO PREPARADA [M]" }
                else { "LIMITE: ACTIVA EL MOTOR EN MARTE" };
            d.draw_text(texto, (sw as i32 - 560) / 2, 192, 24, Color::new(255, 90, 200, parpadeo as u8));
        }
    }
}
