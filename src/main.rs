extern crate rust_kafka_lambda;
extern crate schemafy_core;
extern crate serde;
extern crate serde_json;

use aws_config::BehaviorVersion;
use aws_lambda_events::kafka::KafkaEvent;
use std::env;

use aws_sdk_dynamodb::Client;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use rdkafka::producer::FutureProducer;
use rdkafka::ClientConfig;
use slog::{info, o, Drain, Logger};

use rust_kafka_lambda::adapter::dynamodb_store_converted_product_service::DynamoDbStoreConvertedProductService;
use rust_kafka_lambda::adapter::kafka_notify_update_product_service::KafkaNotifyUpdateProductService;
use rust_kafka_lambda::business::save_converted_product_use_case::SaveConvertedProductUseCase;
use rust_kafka_lambda::handler::lambda_kafka_event_handler::LambdaKafkaEventHandler;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Error> {
    let logger = initialize_logger();

    info!(logger, "Create application context ...");
    let shared_config = aws_config::defaults(BehaviorVersion::v2023_11_09())
        .load()
        .await;
    let client = Client::new(&shared_config);
    let table_name =
        env::var("PRODUCT_TABLE_NAME").expect("Variable PRODUCT_TABLE_NAME is not set!");

    info!(logger, "Create Kafka producer ...");
    let mut kafka_config = ClientConfig::new();
    kafka_config
        .set(
            "bootstrap.servers",
            env::var("KAFKA_BOOTSTRAP_SERVERS")
                .expect("Variable KAFKA_BOOTSTRAP_SERVERS is not set!"),
        )
        .set("security.protocol", "SASL_SSL")
        .set("sasl.mechanism", "PLAIN")
        .set(
            "sasl.username",
            env::var("KAFKA_USERNAME").expect("Variable KAFKA_USERNAME is not set!"),
        )
        .set(
            "sasl.password",
            env::var("KAFKA_PASSWORD").expect("Variable KAFKA_PASSWORD is not set!"),
        );
    let producer: FutureProducer = kafka_config.create()?;
    let topic_name = env::var("UPDATE_CONVERTED_PRODUCT_TOPIC_NAME")
        .expect("Variable UPDATE_CONVERTED_PRODUCT_TOPIC_NAME is not set!");

    info!(logger, "Create save use case ...");
    let save_converted_product_use_case = SaveConvertedProductUseCase {
        log: Logger::new(&logger, o!("logger" => "SaveConvertedProductUseCase")),
        store_converted_product_service: DynamoDbStoreConvertedProductService {
            dynamo_db_client: client,
            table_name,
        },
        notify_update_product_service: KafkaNotifyUpdateProductService {
            producer,
            topic_name,
        },
    };
    info!(logger, "Create handler ...");
    let handler = LambdaKafkaEventHandler {
        log: Logger::new(&logger, o!("logger" => "LambdaKafkaEventHandler")),
        save_converted_product_use_case,
    };
    let func =
        service_fn(|event: LambdaEvent<KafkaEvent>| handler.handle_lambda_kafka_event(event));
    info!(logger, "Start runtime ...");
    lambda_runtime::run(func).await?;
    Ok(())
}

fn initialize_logger() -> Logger {
    let drain = slog_json::Json::default(std::io::stderr()).fuse();
    let drain = slog_envlogger::new(drain).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(
        drain,
        o!("runtime" => "application", "version" => env!("CARGO_PKG_VERSION")),
    );
    slog_scope::set_global_logger(Logger::new(&logger, o!("logger" => "global"))).cancel_reset();
    slog_stdlog::init().expect("Could not initialize standard logger");
    logger
}
