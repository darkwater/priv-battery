extern crate gtk;
extern crate cairo;

use graph::{BatteryGraph, BatteryState};
use gtk::prelude::*;
use self::cairo::Gradient;

pub fn create_linegraph(graph: BatteryGraph) -> gtk::DrawingArea {
    let widget = gtk::DrawingArea::new();
    widget.set_size_request(-1, 100);
    widget.set_hexpand(true);
    widget.set_vexpand(true);

    widget.connect_draw(move |widget, context| {
        let width  = widget.get_allocated_width()  as f64;
        let height = widget.get_allocated_height() as f64;

        let graph_left   = 70.0;
        let graph_top    = 50.0;
        let graph_right  = width - 20.0;
        let graph_bottom = height - 80.0;
        let graph_width  = graph_right - graph_left;
        let graph_height = graph_bottom - graph_top;

        {
            context.set_line_width(1.0);
            context.set_line_cap(cairo::LineCap::Butt);
            context.set_source_rgba(1.0, 1.0, 1.0, 0.2);
            context.translate(0.5, 0.5);

            context.move_to(graph_left, graph_top);
            context.line_to(graph_right, graph_top);

            context.move_to(graph_left, graph_top + graph_height / 2.0);
            context.line_to(graph_right, graph_top + graph_height / 2.0);

            context.move_to(graph_left, graph_bottom);
            context.line_to(graph_right, graph_bottom);

            context.stroke();
            context.translate(-0.5, -0.5);

            context.set_font_size(16.0);
            context.select_font_face("Droid Sans",
                                     cairo::enums::FontSlant::Normal,
                                     cairo::enums::FontWeight::Normal);

            context.set_source_rgba(1.0, 1.0, 1.0, 0.3);
            for &(text, y) in [ ("100%", graph_top),
                               ("50%", graph_top + graph_height / 2.0),
                               ("0%", graph_bottom) ].iter() {
                let extents = context.text_extents(text);
                let x = graph_left - 10.0 - extents.width - extents.x_bearing;
                let y = y - extents.height / 2.0 - extents.y_bearing;

                context.move_to(x, y);
                context.show_text(text);
            }
        }

        let states = graph.get_states();
        draw_graph_line((graph_left, graph_top, graph_width, graph_height), states, &context);

        Inhibit(false)
    });

    widget
}

fn draw_graph_line((x, y, width, height): (f64, f64, f64, f64), states: &[BatteryState], context: &cairo::Context) {
    context.translate(x, y);

    let num_states   = states.len();
    let first_state  = states.first().unwrap();
    let last_state   = states.last().unwrap();

    let sample_threshold = last_state.get_timestamp() - (60 * 5);
    let sample_state     = states.iter().rev().find(|state| state.get_timestamp() < sample_threshold).unwrap();

    let estimated_ratio = {
        let time  = (last_state.get_timestamp() - sample_state.get_timestamp()) as f64;
        let level = (last_state.get_level()     - sample_state.get_level()) as f64;

        level / time
    };

    let estimated_time = last_state.get_level() as f64 / estimated_ratio.abs();

    {
        let estimated_text = {
            let hours   = (estimated_time / 3600.0) as i32;
            let minutes = (estimated_time / 60.0 % 60.0) as i32;

            &format!("{}h {}m left", hours, minutes)
        };

        context.set_source_rgba(1.0, 1.0, 1.0, 0.3);
        context.set_font_size(16.0);
        context.select_font_face("Droid Sans",
                                 cairo::enums::FontSlant::Normal,
                                 cairo::enums::FontWeight::Normal);

        context.set_source_rgba(1.0, 1.0, 1.0, 0.3);
        let extents = context.text_extents(estimated_text);
        let x = width - extents.width - extents.x_bearing;
        let y = height + 10.0 - extents.y_bearing;

        context.move_to(x, y);
        context.show_text(estimated_text);
    }

    let mut states = states.iter();
    let first_state = states.next().unwrap();
    context.move_to(0.0, height - first_state.get_level() as f64 / 100.0 * height);

    let time_window = last_state.get_timestamp() as f64 + estimated_time - first_state.get_timestamp() as f64;

    for state in states {
        let x = (state.get_timestamp() - first_state.get_timestamp()) as f64 / time_window * width;
        let y = height - state.get_level() as f64 / 100.0 * height;

        context.line_to(x, y);
    }

    let (r, g, b, a) = (0.7, 0.9, 1.0, 1.0);
    context.set_source_rgba(r, g, b, a);
    context.set_line_width(4.0);
    context.set_line_cap(cairo::LineCap::Round);
    context.stroke_preserve();

    context.line_to(width, height);
    context.set_line_width(2.0);
    context.set_line_cap(cairo::LineCap::Round);
    context.set_dash(&[0.0, 8.0], 4.0);
    context.stroke_preserve();

    context.line_to(0.0, height);
    context.close_path();

    let gradient = cairo::LinearGradient::new(0.0, 0.0, 0.0, height);
    gradient.add_color_stop_rgba(0.0, 0.7, 0.9, 1.0, 0.2);
    gradient.add_color_stop_rgba(1.0, 0.7, 0.9, 1.0, 0.0);

    context.set_source(&gradient);
    context.fill();

    context.translate(-x, -y);
}
