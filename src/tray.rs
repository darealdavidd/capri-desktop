// Context menu and tray controls for Capri Desktop.

use gtk::prelude::*;
use std::rc::Rc;

pub struct CapriMenu;

impl CapriMenu {
    pub fn build(
        on_teleport: impl Fn() + 'static,
        on_satellite: impl Fn() + 'static,
        on_demo: impl Fn() + 'static,
        on_report: impl Fn() + 'static,
        on_toggle_wander: impl Fn(bool) + 'static,
        on_toggle_mute: impl Fn(bool) + 'static,
        on_change_scale: impl Fn(f64) + 'static,
        on_quit: impl Fn() + 'static,
        current_wander: bool,
        current_muted: bool,
        current_scale: f64,
    ) -> gtk::Menu {
        let menu = gtk::Menu::new();

        let item_title = gtk::MenuItem::with_label("Capri 0.5.0 (Rust / Wayland)");
        item_title.set_sensitive(false);
        menu.append(&item_title);
        menu.append(&gtk::SeparatorMenuItem::new());

        let item_teleport = gtk::MenuItem::with_label("Teleport");
        item_teleport.connect_activate(move |_| on_teleport());
        menu.append(&item_teleport);

        let item_satellite = gtk::MenuItem::with_label("Satellite");
        item_satellite.connect_activate(move |_| on_satellite());
        menu.append(&item_satellite);

        let item_demo = gtk::MenuItem::with_label("Play Demo");
        item_demo.connect_activate(move |_| on_demo());
        menu.append(&item_demo);

        let item_report = gtk::MenuItem::with_label("Mission Report");
        item_report.connect_activate(move |_| on_report());
        menu.append(&item_report);

        menu.append(&gtk::SeparatorMenuItem::new());

        // Wander toggle
        let item_wander = gtk::CheckMenuItem::with_label("Wander");
        item_wander.set_active(current_wander);
        item_wander.connect_toggled(move |item| on_toggle_wander(item.is_active()));
        menu.append(&item_wander);

        // Mute toggle
        let item_mute = gtk::CheckMenuItem::with_label("Mute Audio");
        item_mute.set_active(current_muted);
        item_mute.connect_toggled(move |item| on_toggle_mute(item.is_active()));
        menu.append(&item_mute);

        // Scale submenu
        let item_size = gtk::MenuItem::with_label("Size");
        let size_menu = gtk::Menu::new();

        let scales = [("Small (0.45x)", 0.45), ("Medium (0.6x)", 0.6), ("Large (0.8x)", 0.8)];
        let on_change_scale = Rc::new(on_change_scale);

        for (label, s) in scales {
            let item = gtk::RadioMenuItem::with_label(label);
            if (current_scale - s).abs() < 0.05 {
                item.set_active(true);
            }
            let cb = on_change_scale.clone();
            item.connect_activate(move |it| {
                if it.is_active() {
                    cb(s);
                }
            });
            size_menu.append(&item);
        }
        item_size.set_submenu(Some(&size_menu));
        menu.append(&item_size);

        menu.append(&gtk::SeparatorMenuItem::new());

        let item_quit = gtk::MenuItem::with_label("Quit Capri");
        item_quit.connect_activate(move |_| on_quit());
        menu.append(&item_quit);

        menu.show_all();
        menu
    }
}
