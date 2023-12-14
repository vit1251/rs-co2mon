
use log::LevelFilter;
use env_logger::Builder;

use rs_co2mon::AirQualityMonitor;

fn main() {

    /* Step 1. Initialize report system */
    Builder::new().filter_level(LevelFilter::Debug).init();

    /* Step 2. Create Air Quality Monitor */
    let mut air_mon = AirQualityMonitor::new();
    air_mon.open(); //.unwrap();

    for event in air_mon {
        println!("event = {:?}", event);
    }

}
