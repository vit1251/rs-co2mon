
use log::{info, trace, warn, debug, error};

use hidapi::HidApi;
use hidapi::HidDevice;

const CODE_TAMB : u8 = 0x42; /* Ambient Temperature */
const CODE_CNTR : u8 = 0x50; /* Relative Concentration of CO2 */

fn decode_temperature(w: u16) -> f64 {
    return w as f64 * 0.0625 - 273.15;
}

fn dump(raw: &[u8; 8]) {
    debug!("--- raw ---");
    for i in 0..8 {
        debug!("0x{:02x} ", raw[i]);
    }
    debug!("------");
}

pub struct AirQualityMonitor {
    dev: Option<HidDevice>,    /* USB device hander */
    debug: bool,               /* Debug packet      */
    magic_table: [u8; 8],      /* XOR coding key    */
}

#[derive(Debug)]
pub enum AirQulityEvent {
    AmbientTemperature { temp: f64 },
    RelativeConcentration { value: u16 },
    UnexpectedData,
    ChecksumError,
    UninitializeData,
    UnknownCode,
}

impl Iterator for AirQualityMonitor {
    type Item = AirQulityEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf : [u8; 8] = [0; 8];
        let dev = match &self.dev {
            Some(dev) => dev,
            None => return None,
        };
        let result = dev.read_timeout(&mut buf, 5000);

        /* Decode */
        buf.swap(0, 2);
        buf.swap(1, 4);
        buf.swap(3, 7);
        buf.swap(5, 6);

        for i in 0..8 {
            buf[i] ^= self.magic_table[i];
        }

        let mut result : [u8; 8] = [0; 8];
        let tmp : u8 = buf[7] << 5;
        result[7] = (buf[6] << 5) | (buf[7] >> 3);
        result[6] = (buf[5] << 5) | (buf[6] >> 3);
        result[5] = (buf[4] << 5) | (buf[5] >> 3);
        result[4] = (buf[3] << 5) | (buf[4] >> 3);
        result[3] = (buf[2] << 5) | (buf[3] >> 3);
        result[2] = (buf[1] << 5) | (buf[2] >> 3);
        result[1] = (buf[0] << 5) | (buf[1] >> 3);
        result[0] = tmp | (buf[0] >> 3);

        let magic_word : [u8; 8] = [
            'H' as u8,
            't' as u8,
            'e' as u8,
            'm' as u8,
            'p' as u8,
            '9' as u8,
            '9' as u8,
            'e' as u8,
        ];
        for i in 0..8 {
            let mask : u8 = (magic_word[i] << 4) | (magic_word[i] >> 4);
            result[i] = result[i].wrapping_sub(mask);
        }

        /* Check error message */
        if result[4] != 0x0d {
            warn!("Unexpected data from device (data[4] = {:02x}, want 0x0d)", result[4]);
            return Some(AirQulityEvent::UnexpectedData);
        }

        /* Checksum */
        let r0 : u8 = result[0];
        let r1 : u8 = result[1];
        let r2 : u8 = result[2];
        let r3 : u8 = result[3];
        let checksum = 0u8
            .wrapping_add(r0)
            .wrapping_add(r1)
            .wrapping_add(r2);

        if checksum != r3 {
            warn!("checksum error (0x{:02x}, await 0x{:02x})\n", checksum, r3);
            return Some(AirQulityEvent::ChecksumError);
        }

        /* Debug message on debug mode */
        if self.debug {
            dump(&result);
        }

        /* Decode result */
        let w : u16 = ((result[1] as u16) << 8) + result[2] as u16;
        match r0 {
            CODE_TAMB => {
                let t = decode_temperature(w);
                info!("Ambient Temperature is {}", t);
                return Some(AirQulityEvent::AmbientTemperature { temp: t });
            },
            CODE_CNTR => {
                if w > 3000 {
                    /* Avoid reading spurious (uninitialized?) data */
                    warn!("Reading spurious data. Please wait.");
                    return Some(AirQulityEvent::UninitializeData);
                } else {
                    info!("Relative Concentration of CO2 is {}", w);
                    return Some(AirQulityEvent::RelativeConcentration { value: w });
                }
            },
            _ => {
                debug!("Unknown code {:02x} value {:?}", r0, w);
                return Some(AirQulityEvent::UnknownCode);
            }
        }

        None
    }
}

impl AirQualityMonitor {

    pub fn new() -> Self {
        AirQualityMonitor {
            dev: None,
            debug: false,
            magic_table: [0; 8],
        }
    }

    pub fn open(&mut self) {
        match HidApi::new() {
            Ok(api) => {
                let device = api.open(0x04d9, 0xa052).unwrap();

                let result = device.send_feature_report(&self.magic_table);

                self.dev = Some(device);
            },
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }

}
