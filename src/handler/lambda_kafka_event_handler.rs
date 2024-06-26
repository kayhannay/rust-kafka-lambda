use aws_lambda_events::event::kafka::KafkaEvent;
use lambda_runtime::{Error, LambdaEvent};
use lambda_runtime::tracing::info;
use serde_json::{json, Value};

use crate::business::save_converted_product_use_case::SaveConvertedProductUseCase;
use crate::controller::lambda_kafka_event_controller::lambda_kafka_event_controller;

pub struct LambdaKafkaEventHandler {
    pub save_converted_product_use_case: SaveConvertedProductUseCase,
}

impl LambdaKafkaEventHandler {
    pub async fn handle_lambda_kafka_event(
        &self,
        lambda_event: LambdaEvent<KafkaEvent>,
    ) -> Result<Value, Error> {
        info!("Handle kafka event ...");
        let kafka_event = lambda_event.payload;
        self.save_converted_product_use_case
            .execute(lambda_kafka_event_controller(kafka_event))
            .await;

        Ok(json!({ "message": "Handled Kafka events" }))
    }
}
