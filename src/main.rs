use std::time::Duration;

use color_eyre::Result;
use lab1::{
    config::{self, Configuration},
    domain::AggregatedData,
    file_datasource::{state, FileDatasource},
    reclone,
};
use mqtt::{AsyncClient, ConnectOptionsBuilder};
use paho_mqtt as mqtt;
use tracing::instrument;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

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
    let mut datasource = datasource.start_reading()?;

    tracing::info!("Reading data from the datasource");
    loop {
        interval.tick().await;
        let data: AggregatedData = match datasource.read() {
            Ok(data) => data,
            Err(err) => {
                tracing::error!("Failed to read data from the datasource: {}", err);
                continue;
            }
        };
        tracing::debug!("Sending data to the broker: {data:#?}");
        let message = mqtt::Message::new(topic, serde_json::to_vec(&data)?, 0);
        if let Err(err) = client.publish(message).await {
            tracing::error!("Failed to send message to topic {topic}: {err}")
        };
        tracing::info!("Data sent to the broker");
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
