use std::{borrow::Cow, fs::File, io::BufReader};

use chrono::Utc;
use color_eyre::{
    eyre::{bail, OptionExt},
    Result,
};
use tokio::{fs::File as AsyncFile, io::BufReader as AsyncBufReader};
use tokio_stream::StreamExt;

use crate::{
    domain::{Accelerometer, AggregatedData, Gps},
    KtConvenience,
};

pub struct FileDatasource<State> {
    accelerometer_filename: Cow<'static, str>,
    gps_filename: Cow<'static, str>,
    state: State,
}

/// States of the file data source state machine
pub mod state {
    use super::*;

    use csv_async::AsyncReader;

    /// Initial state of the file data source
    pub struct New;

    /// Sync reading state of the file data source
    pub struct Reading {
        pub accelerometer_reader: csv::Reader<BufReader<File>>,
        pub accelerometer_reader_start: Option<csv::Position>,
        pub gps_reader: csv::Reader<BufReader<File>>,
        pub gps_reader_start: Option<csv::Position>,
    }

    /// Async reading state of the file data source
    pub struct ReadingAsync {
        pub accelerometer_reader: AsyncReader<AsyncBufReader<AsyncFile>>,
        pub gps_reader: AsyncReader<AsyncBufReader<AsyncFile>>,
        pub accelerometer_reader_start: bool,
        pub gps_reader_start: bool,
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

        let accelerometer_reader =
            csv::Reader::from_reader(BufReader::new(File::open(accelerometer_filename.as_ref())?));
        let gps_reader =
            csv::Reader::from_reader(BufReader::new(File::open(gps_filename.as_ref())?));
        Ok(FileDatasource {
            state: state::Reading {
                accelerometer_reader,
                gps_reader,
                accelerometer_reader_start: None,
                gps_reader_start: None,
            },
            accelerometer_filename,
            gps_filename,
        })
    }

    pub async fn start_reading_async(self) -> Result<FileDatasource<state::ReadingAsync>> {
        let Self {
            accelerometer_filename,
            gps_filename,
            ..
        } = self;

        let accelerometer_reader = csv_async::AsyncReader::from_reader(AsyncBufReader::new(
            AsyncFile::open(accelerometer_filename.as_ref()).await?,
        ));
        let gps_reader = csv_async::AsyncReader::from_reader(AsyncBufReader::new(
            AsyncFile::open(gps_filename.as_ref()).await?,
        ));
        Ok(FileDatasource {
            state: state::ReadingAsync {
                accelerometer_reader,
                gps_reader,
                accelerometer_reader_start: false,
                gps_reader_start: false,
            },
            accelerometer_filename,
            gps_filename,
        })
    }
}

impl FileDatasource<state::Reading> {
    pub fn read(&mut self) -> Result<AggregatedData> {
        let Self {
            state:
                state::Reading {
                    accelerometer_reader,
                    gps_reader,
                    accelerometer_reader_start,
                    gps_reader_start,
                },
            ..
        } = self;

        loop {
            let accelerometer: Option<_> = accelerometer_reader
                .deserialize::<Accelerometer>()
                .also(|iter| {
                    if accelerometer_reader_start.is_none() {
                        *accelerometer_reader_start = Some(iter.reader().position().clone());
                    }
                })
                .next()
                .transpose()?
                .also(|v| {
                    if v.is_none() {
                        *accelerometer_reader_start = None;
                    }
                });
            let gps: Option<_> = gps_reader
                .deserialize::<Gps>()
                .also(|iter| {
                    if gps_reader_start.is_none() {
                        *gps_reader_start = Some(iter.reader().position().clone());
                    }
                })
                .next()
                .transpose()?
                .also(|v| {
                    if v.is_none() {
                        *gps_reader_start = None;
                    }
                });

            return match accelerometer.zip(gps) {
                Some((accelerometer, gps)) => {
                    Ok(AggregatedData::new(accelerometer, gps, Utc::now()))
                }
                None => {
                    tracing::debug!("Seeking to the beginning of the files");
                    if accelerometer_reader_start.is_none() || gps_reader_start.is_none() {
                        bail!("Unable to seek to the beginning of the files: start positions are not set")
                    }

                    accelerometer_reader.seek(
                        accelerometer_reader_start
                            .clone()
                            .ok_or_eyre("accelerometer data file is empty")?,
                    )?;

                    gps_reader.seek(
                        gps_reader_start
                            .clone()
                            .ok_or_eyre("gps data file is empty")?,
                    )?;

                    continue;
                }
            };
        }
    }

    pub fn stop_reading(self) -> FileDatasource<state::New> {
        FileDatasource::_new(self.accelerometer_filename, self.gps_filename)
    }
}

impl FileDatasource<state::ReadingAsync> {
    pub async fn read(&mut self) -> Result<AggregatedData> {
        let Self {
            state:
                state::ReadingAsync {
                    accelerometer_reader,
                    gps_reader,
                    accelerometer_reader_start,
                    gps_reader_start,
                },
            ..
        } = self;

        loop {
            let accelerometer: Option<_> = accelerometer_reader
                .records()
                .also(|_| {
                    if !*accelerometer_reader_start {
                        *accelerometer_reader_start = true;
                    }
                })
                .next()
                .await
                .transpose()?;
            let gps: Option<_> = gps_reader
                .records()
                .also(|_| {
                    if !*gps_reader_start {
                        *gps_reader_start = true;
                    }
                })
                .next()
                .await
                .transpose()?;

            return match accelerometer.zip(gps) {
                Some((accelerometer, gps)) => Ok(AggregatedData::new(
                    accelerometer.deserialize(
                        accelerometer_reader
                            .headers()
                            .await?
                            .take_if(|it| !it.is_empty()),
                    )?,
                    gps.deserialize(gps_reader.headers().await?.take_if(|it| !it.is_empty()))?,
                    Utc::now(),
                )),
                None => {
                    tracing::debug!("Seeking to the beginning of the files");
                    if !*accelerometer_reader_start || !*gps_reader_start {
                        bail!("Unable to seek to the beginning of the files: start positions are not set")
                    }

                    accelerometer_reader.rewind().await?;

                    gps_reader.rewind().await?;

                    continue;
                }
            };
        }
    }

    pub fn stop_reading(self) -> FileDatasource<state::New> {
        FileDatasource::_new(self.accelerometer_filename, self.gps_filename)
    }
}
