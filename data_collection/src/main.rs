use chrono::Local;
use clap::Parser;
use rusqlite::{params, Connection};
use std::time::Duration;

use color_eyre::Result;
use mosquitto_rs::*;
use tokio::time::sleep;

/// Lightweight program that performs data collection via MQTT and saves the data to a SQLITE database.
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = String::from("./dev.db"))]
    db_path: String,

    #[arg(short('i'), long, default_value_t = String::from("UNKNOWN_DEVICE"))]
    device_id: String,

    #[arg(short('t'), long, default_value_t = String::from(""))]
    base_topic: String,

    #[arg(short, long, default_value_t = String::from("localhost"))]
    broker_ip: String,
}

struct MqttSensorReading {
    timestamp: u64,
    topic: String,
    value: String,
    device_id: String,
}

impl MqttSensorReading {
    fn new(timestamp: u64, topic: String, value: String, device_id: String) -> MqttSensorReading {
        MqttSensorReading {
            timestamp,
            topic,
            value,
            device_id,
        }
    }
    fn add_to_db(&self, args: &Cli) -> Result<()> {
        let connection = Connection::open(&args.db_path)?;
        connection.execute(
            "INSERT INTO READINGS VALUES (?1, ?2, ?3, ?4);",
            params![self.timestamp, self.topic, self.value, self.device_id],
        )?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();

    let connection = Connection::open(&args.db_path)?;
    connection.execute("
        CREATE TABLE if not exists READINGS (
            timestamp int NOT NULL,
            topic varchar(255) NOT NULL,
            value varchar(255) NOT NULL,
            device_id varchar(255) NOT NULL
        )
        ",
        (),
    )?;

    loop {
        let success = subscribe(&args).await;
        match success {
            Ok(()) => (),
            Err(e) => {
                println!("Something went wrong, trying again soon.\n{}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn subscribe(args: &Cli) -> Result<()> {
    let client = Client::with_auto_id()?;
    client
        .connect(&args.broker_ip, 1883, Duration::from_secs(5), None)
        .await?;

    let subscriptions = client.subscriber();
    let topic = if args.base_topic == ""
        {"/#"} else
        {&format!("/{}/#", &args.base_topic)};

    client.subscribe(topic, QoS::AtMostOnce).await?;

    loop {
        if let Some(subs) = &subscriptions {
            if let Ok(msg) = subs.recv().await {
                if let Event::Message(message) = msg {
                    let reading = MqttSensorReading::new(
                        Local::now().timestamp().try_into()?,
                        message.topic,
                        String::from_utf8(message.payload)?,
                        args.device_id.to_owned(),
                    );
                    reading.add_to_db(&args)?;
                }
            }
        }
    }
}
