extern crate gtk;
extern crate gdk;
extern crate gdk_sys;
extern crate protobuf;
extern crate redis;

mod graph;
mod linegraph;

use graph::{BatteryGraph, BatteryState};
use gtk::prelude::*;
use linegraph::create_linegraph;
use protobuf::Message;
use redis::Commands;
use std::env;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::Path;

/// Returns the contents of a file
fn read_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut string = String::new();
    let mut file   = File::open(path)?;
    let _          = file.read_to_string(&mut string)?;

    Ok(string)
}

/// Returns the location of a battery, if any
fn get_battery_presence() -> io::Result<String> {
    for entry in fs::read_dir("/sys/class/power_supply")? {
        let entry = entry?;
        let dev_type = read_file(entry.path().join("type"))?;

        if dev_type.trim() == "Battery" {
            return Ok(entry.path().to_str().unwrap().to_string());
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "No battery found."))
}

/// Returns battery level and charging status
fn get_battery_status() -> (f32, bool) {
    let battery_presence = get_battery_presence().expect("Couldn't find a battery on this machine.");
    let battery_path = Path::new(&battery_presence);

    // Get battery level
    let file_contents   = read_file(battery_path.join("energy_now")).expect("Couldn't read battery/energy_now.");
    let energy_now: f32 = str::parse(&file_contents.trim())         .expect("Expected a number from battery/energy_now.");

    let file_contents    = read_file(battery_path.join("energy_full")).expect("Couldn't open battery/energy_full.");
    let energy_full: f32 = str::parse(&file_contents.trim())          .expect("Expected a number from battery/energy_full.");

    let level = energy_now * 100.0 / energy_full;

    // Get battery charging status
    let file_contents = read_file(battery_path.join("status")).expect("Couldn't open battery/status.");

    let charging = match file_contents.trim() {
        "Charging" => true,
        "Full"     => true,
        _          => false
    };

    (level, charging)
}

fn prune_old() {
    let con = get_redis_connection();
    let buffer: Vec<u8> = con.get("battery:surface").unwrap();
    let mut graph: BatteryGraph = protobuf::parse_from_bytes(&buffer).unwrap();

    {
        let mut states = graph.mut_states();
        let oldest_timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() as u32 - (60 * 60 * 6); // 6 hours ago
        let newest_timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() as u32 - (60 * 30);     // 30 minutes ago

        states.reverse();

        let mut split_index = 0;
        for (index, state) in states.iter().enumerate() {
            let full = state.get_level() >= 100.0;
            let too_new = state.get_timestamp() > newest_timestamp;
            let too_old = state.get_timestamp() < oldest_timestamp;

            if (full && !too_new) || too_old {
                split_index = index;
                break;
            }
        }

        let new_length = split_index + 1;
        states.truncate(new_length);
        states.reverse();
    }

    let buffer = graph.write_to_bytes().unwrap();
    let a: () = con.set("battery:surface", buffer).unwrap();
    println!("{:?}", a);
}

fn get_redis_connection() -> redis::Connection {
    let client = redis::Client::open("redis://priv-dark-master/").unwrap();
    client.get_connection().unwrap()
}

fn log_state() {
    let (capacity, charging) = get_battery_status();

    let now = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() as u32;

    let mut graph = BatteryGraph::new();
    let mut state = BatteryState::new();

    state.set_timestamp(now);
    state.set_level(capacity);
    state.set_charging(charging);

    graph.set_states(protobuf::RepeatedField::from_vec(vec![state]));

    let buffer = graph.write_to_bytes().unwrap();

    let con = get_redis_connection();
    let _: () = con.append("battery:surface", buffer).unwrap();
}

fn show_window() {
    let con = get_redis_connection();
    let buffer: Vec<u8> = con.get("battery:surface").unwrap();
    let graph: BatteryGraph = protobuf::parse_from_bytes(&buffer).unwrap();

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_name("priv-battery");
    window.set_type_hint(gdk::WindowTypeHint::Dialog);
    window.set_decorated(false);

    let screen = window.get_screen().unwrap();
    let monitor_id = screen.get_primary_monitor();
    let monitor = screen.get_monitor_geometry(monitor_id);

    let css_provider = gtk::CssProvider::new();
    let _ = css_provider.load_from_data("
    window {
        background-color: #1d1f21;
    }

    #header {
        color: #efefef;
        padding: 15px 20px;
        padding-bottom: 10px;
    }

    #subheader {
        color: #abdbfb;
        padding: 0px 20px;
        padding-bottom: 15px;
        /* margin-bottom: 20px; */
    }
    ");
    gtk::StyleContext::add_provider_for_screen(&screen, &css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    window.set_size_request(500, 400);

    let grid = gtk::Grid::new();
    grid.set_hexpand(true);
    grid.set_vexpand(true);
    grid.set_orientation(gtk::Orientation::Vertical);
    window.add(&grid);

    let header = gtk::Label::new("Battery");
    header.set_name("header");
    header.set_xalign(0.0);

    let subheader = gtk::Label::new("dark-surface");
    subheader.set_name("subheader");
    subheader.set_xalign(0.0);

    grid.add(&header);
    grid.add(&subheader);

    let linegraph = create_linegraph(graph);
    grid.add(&linegraph);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}

fn main() {
    match env::args().skip(1).next().expect("Expected an action").as_ref() {
        "log"    => return log_state(),
        "prune"  => return prune_old(),
        "window" => return show_window(),
        arg @ _  => panic!("Unrecognized action {:?}", arg)
    }
}
