
use std::env;

use log::LevelFilter;
use env_logger::Builder;

use telegraf::{Client, Point};

use rs_co2mon::AirQualityMonitor;
use rs_co2mon::AirQulityEvent::AmbientTemperature;
use rs_co2mon::AirQulityEvent::RelativeConcentration;

fn main() {

    let mut enable_telegraf: bool = false;
    let mut enable_debug: bool = false;
    let mut verbose: bool = true;

    let mut c: Option<Client> = None;

    /* Step 1. Parse arguments*/
    for argument in env::args() {
        if argument == "--debug" {
            enable_debug = true;
        }
        if argument == "--telegraf" {
            enable_telegraf = true;
            c = Some(Client::new("tcp://localhost:8094").unwrap());
        }
        if argument == "--help" {
            // TODO - write help message...
        }
    }

    /* Step 2. Initialize report system */
    if enable_debug {
        Builder::new().filter_level(LevelFilter::Debug).init();
    }

    /* Step 3. Create Air Quality Monitor */
    let mut air_mon = AirQualityMonitor::new();
    air_mon.open(); //.unwrap();

    for event in air_mon {
        if let Some(ref mut conn) = c {
            match event {
                AmbientTemperature { temp } => {
                    let p = Point::new(
                        String::from("co2monitor"),
                        vec![
//                            (String::from("name"), String::from(""))
                        ],
                        vec![
                            (String::from("ambient_temperature"), Box::new(temp)),
                        ],
                        None,
                    );
                    conn.write_point(&p).unwrap();
                },
                RelativeConcentration { value } => {
                    let p = Point::new(
                        String::from("co2monitor"),
                        vec![
//                            (String::from("name"), String::from("relative_concentration"))
                        ],
                        vec![
                            (String::from("relative_concentration"), Box::new(value)),
                        ],
                        None,
                    );
                    conn.write_point(&p).unwrap();
                },
                _ => {

                },
            }
        }
        if verbose {
            println!("event = {:?}", event);
        }
    }

}
