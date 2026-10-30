// Main GTK Wayland application, layer-shell integration, game loop,
// and input region updating for seamless click-through.

use crate::brain::Brain;
use crate::bridge::{BridgeServer, HookEvent, DEFAULT_PORT};
use crate::capri::Capri;
use crate::chip::{Chip, Mask};
use crate::config::Config;
use crate::fx::FxLayer;
use crate::mission::MissionTracker;
use crate::panel::Panel;
use crate::sprite::{SpriteRenderer, SPRITE_HEIGHT, SPRITE_PAD, SPRITE_WIDTH};
use crate::tray::CapriMenu;
use crate::util::current_time_ms;
use crate::voice::Voice;

use gdk::prelude::*;
use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};

pub struct AppState {
    pub capri: Capri,
    pub renderer: SpriteRenderer,
    pub fx: FxLayer,
    pub panel: Panel,
    pub mask: Mask,
    pub chip: Chip,
    pub brain: Brain,
    pub voice: Voice,
    pub mission: MissionTracker,
    pub config: Config,
    pub rx: Receiver<HookEvent>,
    pub last_tick_ms: f64,
}

pub fn run(config: Config) -> Result<(), anyhow::Error> {
    gtk::init()?;

    let (tx, rx) = channel::<HookEvent>();
    BridgeServer::start(DEFAULT_PORT, tx);

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("capri");
    let sprite_path = assets_dir.join("capri-full.png");
    let renderer = SpriteRenderer::load(&sprite_path)?;

    let fx = FxLayer::new(&assets_dir);
    let panel = Panel::new(&assets_dir);
    let mask = Mask::new(&assets_dir);
    let chip = Chip::new();
    let brain = Brain::new(&config.resolved_lang(), &config.assistant);
    let voice = Voice::new();
    let mission = MissionTracker::load();

    // Create Layer Shell window
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.init_layer_shell();
    window.set_layer(gtk_layer_shell::Layer::Overlay);
    window.set_keyboard_interactivity(false);
    window.set_exclusive_zone(-1);
    window.set_namespace("capri-desktop");

    // Anchor to all 4 edges to span full screen
    window.set_anchor(gtk_layer_shell::Edge::Top, true);
    window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
    window.set_anchor(gtk_layer_shell::Edge::Left, true);
    window.set_anchor(gtk_layer_shell::Edge::Right, true);

    // Transparent RGBA visual
    if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&window) {
        if let Some(visual) = screen.rgba_visual() {
            window.set_visual(Some(&visual));
        }
    }
    window.set_app_paintable(true);

    // Determine screen dimensions
    let display = gdk::Display::default().expect("Wayland display needed");
    let monitor = display.primary_monitor().or_else(|| display.monitor(0));
    let (screen_w, screen_h) = if let Some(m) = monitor {
        let geom = m.geometry();
        (geom.width() as f64, geom.height() as f64)
    } else {
        (1920.0, 1080.0)
    };

    let mut capri = Capri::new(screen_w, screen_h, config.scale);
    capri.wander = config.wander;

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.add_events(
        gdk::EventMask::BUTTON_PRESS_MASK
            | gdk::EventMask::BUTTON_RELEASE_MASK
            | gdk::EventMask::POINTER_MOTION_MASK,
    );

    let state = Rc::new(RefCell::new(AppState {
        capri,
        renderer,
        fx,
        panel,
        mask,
        chip,
        brain,
        voice,
        mission,
        config,
        rx,
        last_tick_ms: current_time_ms(),
    }));

    // Setup Draw callback
    {
        let state = state.clone();
        drawing_area.connect_draw(move |_widget, cr| {
            let mut s = state.borrow_mut();
            let now = current_time_ms();

            // Clear with full transparency
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.set_operator(cairo::Operator::Source);
            let _ = cr.paint();
            cr.set_operator(cairo::Operator::Over);

            // Draw Effects beneath Capri
            s.fx.update_and_draw(cr, now);

            // Draw Capri Sprite
            if !s.capri.stowed && !s.capri.dormant {
                s.renderer.draw(
                    cr,
                    s.capri.pos.0,
                    s.capri.pos.1,
                    s.capri.scale,
                    s.capri.facing,
                    s.capri.bob,
                    s.capri.tilt,
                    s.capri.sx,
                    s.capri.sy,
                    &s.capri.p,
                );
            }

            // Draw Biochip / Mask
            s.chip.draw(cr, now);
            s.mask.draw(cr, now);

            // Draw Transmission Panel
            s.panel.draw(cr, now);

            glib::Propagation::Proceed
        });
    }

    // Setup Mouse interactions
    {
        let state = state.clone();
        let window_weak = window.downgrade();

        drawing_area.connect_button_press_event(move |_widget, event| {
            let (mx, my) = event.position();
            let mut s = state.borrow_mut();

            if event.button() == 1 {
                // Left click
                if s.panel.hit_test(mx, my) {
                    s.panel.hide();
                    return glib::Propagation::Stop;
                }

                if s.chip.hit_test(mx, my) {
                    s.capri.stowed = false;
                    s.chip.visible = false;
                    s.capri.wake();
                    return glib::Propagation::Stop;
                }

                if s.mask.hit_test(mx, my) {
                    s.capri.wake();
                    s.mask.hide();
                    return glib::Propagation::Stop;
                }

                // Check click on Capri
                if s.renderer.hit_test(
                    mx,
                    my,
                    s.capri.pos.0,
                    s.capri.pos.1,
                    s.capri.scale,
                    s.capri.facing,
                    s.capri.bob,
                    s.capri.p.sit,
                ) {
                    s.capri.start_drag(mx, my);
                    return glib::Propagation::Stop;
                }
            } else if event.button() == 3 {
                // Right click: Popup Context Menu
                if let Some(_win) = window_weak.upgrade() {
                    let state_clone = state.clone();
                    let menu = CapriMenu::build(
                        {
                            let st = state_clone.clone();
                            move || {
                                let mut s = st.borrow_mut();
                                let s = &mut *s;
                                let x = crate::util::rand_range(150.0, s.capri.screen_w - 200.0);
                                let floor = s.capri.floor;
                                s.capri.teleport((x, floor), &mut s.fx, &s.voice);
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move || {
                                let mut s = st.borrow_mut();
                                s.capri.next_gaze = 0.0;
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move || {
                                let mut s = st.borrow_mut();
                                s.panel.say("info", "Replaying system demo sequence...", "DEMO", Some("laugh"), None, 4000.0, current_time_ms());
                                s.capri.perform("victory", 1.0);
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move || {
                                let mut s = st.borrow_mut();
                                let m = &s.mission.day;
                                let summary = format!("{} missions · {} actions · {} errors", m.missions, m.actions, m.errors);
                                s.panel.say("done", "Rapport de mission du jour prêt.", "MISSION", Some("smug"), Some(&summary), 8000.0, current_time_ms());
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move |wander| {
                                let mut s = st.borrow_mut();
                                s.capri.wander = wander;
                                s.config.wander = wander;
                                let _ = s.config.save();
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move |muted| {
                                let mut s = st.borrow_mut();
                                s.voice.muted = muted;
                                s.config.muted = muted;
                                let _ = s.config.save();
                            }
                        },
                        {
                            let st = state_clone.clone();
                            move |scale| {
                                let mut s = st.borrow_mut();
                                s.capri.scale = scale;
                                s.config.scale = scale;
                                let _ = s.config.save();
                            }
                        },
                        move || {
                            gtk::main_quit();
                        },
                        s.capri.wander,
                        s.voice.muted,
                        s.capri.scale,
                    );
                    menu.popup_at_pointer(Some(event));
                    return glib::Propagation::Stop;
                }
            }

            glib::Propagation::Proceed
        });
    }

    {
        let state = state.clone();
        drawing_area.connect_motion_notify_event(move |_widget, event| {
            let (mx, my) = event.position();
            let mut s = state.borrow_mut();
            let s = &mut *s;
            if s.capri.dragging {
                s.capri.drag_move(mx, my);
            } else {
                s.capri.handle_mouse_hover(mx, my, &s.renderer);
            }
            glib::Propagation::Proceed
        });
    }

    {
        let state = state.clone();
        drawing_area.connect_button_release_event(move |_widget, event| {
            if event.button() == 1 {
                let mut s = state.borrow_mut();
                let s = &mut *s;
                if s.capri.dragging {
                    s.capri.end_drag(&s.voice);
                }
            }
            glib::Propagation::Proceed
        });
    }

    window.add(&drawing_area);
    window.show_all();

    // Wayland Input Click-Through Region Update Function
    let update_input_region = {
        let window_weak = window.downgrade();
        let state = state.clone();

        move || {
            if let Some(win) = window_weak.upgrade() {
                if let Some(gdk_win) = win.window() {
                    let s = state.borrow();
                    let region = cairo::Region::create();

                    // 1. Capri Bounding Box
                    if !s.capri.stowed && !s.capri.dormant {
                        let scale = s.capri.scale;
                        let pad = SPRITE_PAD * scale;
                        let w = (SPRITE_WIDTH * scale + pad * 2.0) as i32;
                        let h = (SPRITE_HEIGHT * scale + pad * 2.0) as i32;
                        let x = (s.capri.pos.0 - (SPRITE_PAD + 128.0) * scale) as i32;
                        let y = (s.capri.pos.1 - (SPRITE_PAD + 418.0) * scale + s.capri.bob) as i32;
                        let rect = cairo::RectangleInt::new(x, y, w, h);
                        let _ = region.union_rectangle(&rect);
                    }

                    // 2. Transmission Panel
                    if s.panel.visible {
                        let rect = cairo::RectangleInt::new(
                            s.panel.x as i32,
                            s.panel.y as i32,
                            s.panel.width as i32,
                            s.panel.height as i32,
                        );
                        let _ = region.union_rectangle(&rect);
                    }

                    // 3. Biochip
                    if s.chip.visible {
                        let rect = cairo::RectangleInt::new(
                            s.chip.x as i32,
                            s.chip.y as i32,
                            s.chip.width as i32,
                            s.chip.height as i32,
                        );
                        let _ = region.union_rectangle(&rect);
                    }

                    // 4. Mask
                    if s.mask.visible {
                        let radius = (40.0 * s.mask.scale) as i32;
                        let rect = cairo::RectangleInt::new(
                            (s.mask.x as i32) - radius,
                            (s.mask.y as i32) - radius,
                            radius * 2,
                            radius * 2,
                        );
                        let _ = region.union_rectangle(&rect);
                    }

                    // Apply to GDK Window -> Translates directly to wl_surface_set_input_region!
                    gdk_win.input_shape_combine_region(&region, 0, 0);
                }
            }
        }
    };

    // Main 60 FPS Game Loop
    {
        let state = state.clone();
        let drawing_area_weak = drawing_area.downgrade();
        let update_input_region = update_input_region;

        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let now = current_time_ms();
            let mut s = state.borrow_mut();
            let dt = ((now - s.last_tick_ms) / 1000.0).clamp(0.001, 0.05);
            s.last_tick_ms = now;

            let s = &mut *s;
            let capri = &mut s.capri;
            let renderer = &s.renderer;
            let fx = &mut s.fx;
            let panel = &mut s.panel;
            let mask = &mut s.mask;
            let brain = &mut s.brain;
            let voice = &s.voice;
            let mission = &mut s.mission;
            let rx = &s.rx;

            // 1. Process incoming hook events from bridge
            let events: Vec<HookEvent> = rx.try_iter().collect();
            for evt in events {
                let capri_pos = capri.pos;
                let mut act = capri.act.take();
                let mut mode = capri.mode.clone();
                let mut uplink = capri.uplink_due;

                brain.handle(
                    &evt,
                    panel,
                    voice,
                    fx,
                    mission,
                    capri_pos,
                    &mut act,
                    &mut mode,
                    &mut uplink,
                );

                capri.act = act;
                capri.mode = mode;
                capri.uplink_due = uplink;
                capri.wake();
            }

            // 2. Thinking watchdog tick
            let capri_pos = capri.pos;
            let mut act = capri.act.take();
            brain.tick_thinking(panel, capri_pos, &mut act, now);
            capri.act = act;

            // 3. Capri behavior tick
            capri.tick(dt, fx, voice, renderer);

            // 4. Update Panel position relative to head
            let head = capri.head_pos();
            let screen_w = capri.screen_w;
            let lean = if capri.facing > 0.0 { -1.0 } else { 1.0 };
            panel.update_pos(head.0, head.1, screen_w, lean);

            // 5. Update Mask if dormant
            let scale = capri.scale;
            if capri.dormant {
                mask.show(head.0, head.1, scale * 1.1);
            } else {
                mask.hide();
            }

            // 6. Request redraw
            if let Some(da) = drawing_area_weak.upgrade() {
                da.queue_draw();
            }

            // 7. Update input region for Wayland click-through
            update_input_region();

            glib::ControlFlow::Continue
        });
    }

    gtk::main();
    Ok(())
}
