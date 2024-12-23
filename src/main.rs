
use std::env;
use std::time::Duration;

use log::{LevelFilter, debug};
use env_logger::Builder;

use telegraf::{Client, Point};

use rs_co2mon::AirQulityEvent;
use rs_co2mon::OpenOptions;
use rs_co2mon::AirQulityEvent::AmbientTemperature;
use rs_co2mon::AirQulityEvent::RelativeConcentration;

fn report(mon: &mut Monitor, event: &AirQulityEvent) {

    if let Some(ref mut conn) = mon.telegraf_client {

    match *event {
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
}

struct Monitor {
    telegraf_enable: bool,           /* Delivery metric on Telegraf proxy */
    telegraf_client: Option<Client>, /* Telegraf client                   */
    enable_debug: bool,              /* Show debug output                 */
}

impl Monitor {
    fn new() -> Self {
        Monitor {
            telegraf_enable: false,
            telegraf_client: None,
            enable_debug: false,
        }
    }
}

fn main() {

    let mut mon: Monitor = Monitor::new();

    /* Step 1. Parse arguments*/
    for argument in env::args() {
        if argument == "--debug" {
            mon.enable_debug = true;
        }
        if argument == "--telegraf" {
            mon.telegraf_enable = true;
            mon.telegraf_client = Some(Client::new("tcp://127.0.0.1:8094").unwrap());
        }
    }

    /* Step 2. Initialize debug system */
    if mon.enable_debug {
        Builder::new().filter_level(LevelFilter::Debug).init();
    }

    /* Step 3. Create Air Quality Monitor */
    let mut sensor = OpenOptions::new()
        .with_key([0x62, 0xea, 0x1d, 0x4f, 0x14, 0xfa, 0xe5, 0x6c])
        .timeout(Some(Duration::from_secs(5)))
        .debug(mon.enable_debug)
        .open()
        .unwrap();

    /* Step 4. Process sensor monitoring */
    loop {
        if let Some(event) = sensor.read() {
            if mon.enable_debug {
                debug!("event = {:?}", event);
            }
            report(&mut mon, &event);
        }
    }

}
