
use std::time::Duration; 

use log::{warn, info, debug};

use hidapi::HidApi;
use hidapi::HidDevice;

mod error;

const CODE_HUMD : u8 = 0x41; /* Humidity                      */
const CODE_TAMB : u8 = 0x42; /* Ambient Temperature           */
const CODE_CNTR : u8 = 0x50; /* Relative Concentration of CO2 */

fn decode_humidity(w: u16) -> f64 {
    w as f64 / 100.0
}

fn decode_temperature(w: u16) -> f64 {
    w as f64 / 16.0 - 273.15
}

fn dump(raw: &[u8; 8]) {
    debug!("--- raw ---");
    for i in 0..8 {
        debug!("0x{:02x} ", raw[i]);
    }
    debug!("------");
}

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Sensor {
    dev: HidDevice,            /* USB device hander */
    debug: bool,               /* Debug packet      */
    decode: bool,              /* Use decode        */
    key: [u8; 8],              /* Key               */
    timeout: Option<Duration>, /* Timeout           */
}

#[derive(Debug)]
pub enum AirQulityEvent {
    AmbientTemperature { temp: f64 },
    RelativeConcentration { value: u16 },
    Humidity { value: f64 },
    UnexpectedData([u8; 8]),
    WrongPacket,
    ChecksumError,
    UninitializeData,
    UnknownCode([u8; 8]),
}

fn decrypt(mut data: [u8; 8], key: [u8; 8]) -> [u8; 8] {

    data.swap(0, 2);
    data.swap(1, 4);
    data.swap(3, 7);
    data.swap(5, 6);

    for (r, k) in data.iter_mut().zip(key.iter()) {
        *r ^= k;
    }

    let tmp : u8 = data[7] << 5;
    data[7] = (data[6] << 5) | (data[7] >> 3);
    data[6] = (data[5] << 5) | (data[6] >> 3);
    data[5] = (data[4] << 5) | (data[5] >> 3);
    data[4] = (data[3] << 5) | (data[4] >> 3);
    data[3] = (data[2] << 5) | (data[3] >> 3);
    data[2] = (data[1] << 5) | (data[2] >> 3);
    data[1] = (data[0] << 5) | (data[1] >> 3);
    data[0] = tmp | (data[0] >> 3);

    for (r, m) in data.iter_mut().zip(b"Htemp99e".iter()) {
        *r = r.wrapping_sub(m << 4 | m >> 4);
    }

    data
}


impl Sensor {

    fn open(options: &OpenOptions) -> Result<Self> {
        let hidapi = HidApi::new()?;

        const VID: u16 = 0x04d9;
        const PID: u16 = 0xa052;

        let device = hidapi.open(VID, PID)?;

        let info = device.get_device_info().unwrap();
        let release_number = info.release_number();
        info!("Device: release-number = {:#04x}", release_number);
        let decode = if release_number > 0x0100 {
            false
        } else {
            true
        };

        let key = options.key;

        let frame = {
            let mut frame = [0; 9];
            frame[1..9].copy_from_slice(&key);
            frame
        };

        if let Ok(result) = device.send_feature_report(&frame) {
            debug!("init = {:?}", result);
            // TODO - process send feature error...
        }

        let debug = options.debug;

        Ok(Self {
            dev: device,
            debug: debug,
            decode: decode,
            key: key,
            timeout: None,
        })
    }

    pub fn read(&mut self) -> Option<AirQulityEvent> {

        let mut data : [u8; 8] = [0; 8];

        let timeout = self.timeout
            .unwrap_or(Duration::from_secs(5))
            .as_millis() as i32;

        if let Ok(size) = self.dev.read_timeout(&mut data, timeout) {
            if size == 8 {
            } else {
                debug!("read_timeout: size = {:?}", size);
                return Some(AirQulityEvent::WrongPacket);
            }
        } else {
            return None;
        }

        /* Step 2. Decode */
        let data = if self.decode {
            decrypt(data, self.key)
        } else {
            data
        };

        /* Check error message */
        if data[4] != 0x0d {
            dump(&data);
            warn!("Unexpected data from device (data[4] = {:02x}, want 0x0d)", data[4]);
            return Some(AirQulityEvent::UnexpectedData(data));
        }

        /* Checksum */
        let r0 : u8 = data[0];
        let r1 : u8 = data[1];
        let r2 : u8 = data[2];
        let r3 : u8 = data[3];
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
            dump(&data);
        }

        /* Decode result */
        let w : u16 = ((data[1] as u16) << 8) + data[2] as u16;
        if self.debug {
            debug!("code = {} value = {}", r0, w);
        }
        match r0 {
            CODE_HUMD => {
                let h = decode_humidity(w);
                Some(AirQulityEvent::Humidity{ value: h })
            },
            CODE_TAMB => {
                let t = decode_temperature(w);
                info!("Ambient Temperature is {}", t);
                Some(AirQulityEvent::AmbientTemperature { temp: t })
            },
            CODE_CNTR => {
                if w > 3000 {
                    /* Avoid reading spurious (uninitialized?) data */
                    warn!("Reading spurious data. Please wait.");
                    Some(AirQulityEvent::UninitializeData)
                } else {
                    info!("Relative Concentration of CO2 is {}", w);
                    Some(AirQulityEvent::RelativeConcentration { value: w })
                }
            },
            _ => {
                debug!("Unknown code {:02x} value {:?}", r0, w);
                Some(AirQulityEvent::UnknownCode(data))
            }
        }
    }

}

#[derive(Debug, Clone)]
pub struct OpenOptions {
//    path_type: DevicePathType,
    key: [u8; 8],
    debug: bool,
    timeout: Option<Duration>,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenOptions {
    pub fn new() -> Self {
        Self {
//            path_type: DevicePathType::Id,
            key: [0; 8],
            debug: false,
            timeout: Some(Duration::from_secs(5)),
        }
    }

    pub fn debug(&mut self, yesno: bool) -> &mut Self {
        self.debug = yesno;
        self
    }

    pub fn with_key(&mut self, key: [u8; 8]) -> &mut Self {
        self.key = key;
        self
    }

    pub fn timeout(&mut self, timeout: Option<Duration>) -> &mut Self {
        self.timeout = timeout;
        self
    }

    pub fn open(&self) -> Result<Sensor> {
        Sensor::open(self)
    }

}
