use std::time::Duration;

use color_eyre::Result;
use lab1::{
    config::{self, Configuration},
    domain::AggregatedData,
    file_datasource::{state, FileDatasource},
    reclone, FileStdoutWriter,
};
use mqtt::{AsyncClient, ConnectOptionsBuilder};
use paho_mqtt as mqtt;
use tracing::instrument;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(FileStdoutWriter::new(
        tracing_appender::rolling::never("./log", "lab1.log"),
    ));
    SubscriberInitExt::try_init(
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(non_blocking)
            .with_thread_ids(true),
    )?;

    let config = Configuration::try_read()?;
    let client = connect_mqtt(config.mqtt().to_owned()).await?;

    let datasource = FileDatasource::new("./data/accelerometer.csv", "./data/gps.csv");
    let result = publish(client, config.mqtt().topic(), datasource, config.delay()).await;

    result.map_err(Into::into)
}

#[instrument(skip(client, datasource))]
async fn publish(
    client: AsyncClient,
    topic: &str,
    datasource: FileDatasource<state::New>,
    delay: Duration,
) -> Result<()> {
    let mut interval = tokio::time::interval(delay);
    let datasource = datasource.start_reading_async().await?;

    let (data_reader_sender, mut data_reader_receiver) =
        tokio::sync::mpsc::channel::<AggregatedData>(7);

    tokio::spawn(read_data(datasource, data_reader_sender));

    while let Some(data) = data_reader_receiver.recv().await {
        tracing::debug!("Data received from the channel. Sending to the broker: {data:#?}");
        let message = mqtt::Message::new(topic, serde_json::to_vec(&data)?, 0);
        if let Err(err) = client.publish(message).await {
            tracing::error!("Failed to send message to topic {topic}: {err}")
        } else {
            tracing::info!("Data sent to the broker");
        }
        interval.tick().await;
    }
    tracing::info!("No more data");

    Ok(())
}

#[instrument(skip(datasource, data_reader_sender))]
async fn read_data(
    mut datasource: FileDatasource<state::ReadingAsync>,
    data_reader_sender: tokio::sync::mpsc::Sender<AggregatedData>,
) {
    tracing::info!("Reading data from the datasource");
    loop {
        let data: AggregatedData = match datasource.read().await {
            Ok(data) => data,
            Err(err) => {
                tracing::error!("Failed to read data from the datasource: {}", err);
                continue;
            }
        };
        tracing::debug!("Sending data to the channel: {data:#?}");
        if let Err(err) = data_reader_sender.send(data).await {
            tracing::error!("Failed to send data to the receiver: {}", err);
        }
    }
}

#[instrument(skip(config))]
async fn connect_mqtt(config: config::Mqtt) -> Result<AsyncClient> {
    let client = mqtt::AsyncClient::new(format!(
        "tcp://{}:{}",
        config.brocker_host(),
        config.brocker_port()
    ))?;

    client
        .connect_with_callbacks(
            ConnectOptionsBuilder::new().finalize(),
            {
                reclone!(config);
                move |_, _| {
                    tracing::info!(
                        "Connected to the broker ({}:{})",
                        config.brocker_host(),
                        config.brocker_port()
                    );
                }
            },
            move |_, _, rc| {
                tracing::info!(
                    "Failed to connect to the broker ({}:{}), return code {rc}",
                    config.brocker_host(),
                    config.brocker_port()
                );
            },
        )
        .await?;

    Ok(client)
}
