use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Accelerometer {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Gps {
    longitude: f64,
    latitude: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AggregatedData {
    accelerometer: Accelerometer,
    gps: Gps,
    time: DateTime<Utc>,
}

impl Accelerometer {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn z(&self) -> i32 {
        self.z
    }
}

impl Gps {
    pub fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    pub fn latitude(&self) -> f64 {
        self.latitude
    }
}

impl AggregatedData {
    pub fn new(accelerometer: Accelerometer, gps: Gps, time: DateTime<Utc>) -> Self {
        Self {
            accelerometer,
            gps,
            time,
        }
    }

    pub fn accelerometer(&self) -> Accelerometer {
        self.accelerometer
    }

    pub fn gps(&self) -> Gps {
        self.gps
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }
}
