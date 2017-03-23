extern crate gtk;
extern crate cairo;

use graph::{BatteryGraph, BatteryState};
use gtk::prelude::*;

pub fn create_linegraph(graph: BatteryGraph) -> gtk::DrawingArea {
    let widget = gtk::DrawingArea::new();
    widget.set_size_request(-1, 100);
    widget.set_hexpand(true);
    widget.set_vexpand(true);

    widget.connect_draw(move |widget, context| {
        let width  = widget.get_allocated_width()  as f64;
        let height = widget.get_allocated_height() as f64;

        context.set_font_size(12.0);
        context.select_font_face("Droid Sans Mono",
                                 cairo::enums::FontSlant::Normal,
                                 cairo::enums::FontWeight::Normal);

        // let extents = context.text_extents(text);
        // let x = width  / 2.0 - extents.width  / 2.0 - extents.x_bearing;
        // let y = height / 2.0 - extents.height / 2.0 - extents.y_bearing;

        let (r, g, b, a) = (1.0, 1.0, 1.0, 0.7);
        context.set_source_rgba(r, g, b, a);

        context.move_to(-100.0, 0.0);

        let states          = graph.get_states();
        let num_states      = states.len();
        let first_timestamp = states.first().unwrap().get_timestamp() as f64;
        let last_timestamp  = states.last().unwrap().get_timestamp() as f64;

        for state in states {
            let x = (state.get_timestamp() as f64 - first_timestamp) / (last_timestamp - first_timestamp) * width;
            let y = height - state.get_level() as f64 / 100.0 * height;

            context.line_to(x, y);
            println!("{:?} \t {:.2}, {:.2}", state, x, y);
        }

        context.stroke();

        // context.move_to(x, y);
        // context.show_text(text);

        // let used_width = extents.width + 30.0;

        Inhibit(false)
    });

    widget
}
