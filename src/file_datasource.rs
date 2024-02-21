use std::{borrow::Cow, fs::File};

use chrono::Utc;
use color_eyre::Result;

use crate::domain::{Accelerometer, AggregatedData, Gps};

pub struct FileDatasource<State> {
    accelerometer_filename: Cow<'static, str>,
    gps_filename: Cow<'static, str>,
    state: State,
}

pub mod state {
    use std::fs::File;

    pub struct New;
    pub struct Reading {
        pub accelerometer_reader: csv::Reader<File>,
        pub gps_reader: csv::Reader<File>,
    }
}

impl FileDatasource<state::New> {
    #[inline(always)]
    pub fn new<S1, S2>(accelerometer_filename: S1, gps_filename: S2) -> Self
    where
        S1: Into<Cow<'static, str>>,
        S2: Into<Cow<'static, str>>,
    {
        Self::_new(accelerometer_filename.into(), gps_filename.into())
    }

    fn _new(accelerometer_filename: Cow<'static, str>, gps_filename: Cow<'static, str>) -> Self {
        Self {
            accelerometer_filename,
            gps_filename,
            state: state::New,
        }
    }

    pub fn start_reading(self) -> Result<FileDatasource<state::Reading>> {
        let Self {
            accelerometer_filename,
            gps_filename,
            ..
        } = self;

        Ok(FileDatasource {
            state: state::Reading {
                accelerometer_reader: csv::Reader::from_reader(File::open(
                    &*accelerometer_filename,
                )?),
                gps_reader: csv::Reader::from_reader(File::open(&*gps_filename)?),
            },
            accelerometer_filename,
            gps_filename,
        })
    }
}

impl FileDatasource<state::Reading> {
    pub fn read(&mut self) -> Result<Option<AggregatedData>> {
        let Self {
            state:
                state::Reading {
                    accelerometer_reader,
                    gps_reader,
                },
            ..
        } = self;

        let accelerometer: Option<Accelerometer> =
            accelerometer_reader.deserialize().next().transpose()?;
        let gps: Option<Gps> = gps_reader.deserialize().next().transpose()?;

        let aggregated = accelerometer.zip(gps);

        Ok(aggregated
            .map(|(accelerometer, gps)| AggregatedData::new(accelerometer, gps, Utc::now())))
    }

    pub fn stop_reading(self) -> FileDatasource<state::New> {
        FileDatasource::_new(self.accelerometer_filename, self.gps_filename)
    }
}
