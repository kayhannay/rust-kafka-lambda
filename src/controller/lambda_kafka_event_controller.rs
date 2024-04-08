use crate::domain::{product::Product, product_event::ProductEvent};
use aws_lambda_events::event::kafka::KafkaEvent;
use lambda_runtime::tracing::info;
use lib_base64::Base64;

pub fn lambda_kafka_event_controller(kafka_event: KafkaEvent) -> Vec<ProductEvent> {
    info!("Handle Kafka event in controller ...");

    kafka_event
        .records
        .iter()
        .flat_map(|it| it.1)
        .map(|record| {
            let mut product: Option<Product> = None;
            if record.value.is_some() {
                let json_string = record
                    .value
                    .as_ref()
                    .unwrap()
                    .decode()
                    .expect("Could not decode event data");
                product = serde_json::from_str(&json_string).ok();
            }
            ProductEvent {
                product_id: record
                    .key
                    .clone()
                    .expect("No product id provided as key in the KafkaEvent!")
                    .decode()
                    .expect("Could not decode event key"),
                product,
            }
        })
        .collect::<Vec<ProductEvent>>()
}
