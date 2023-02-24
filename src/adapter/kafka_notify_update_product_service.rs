use std::time::Duration;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use async_trait::async_trait;
use crate::services::notify_update_product_service::{CommandType, NotifyUpdateProductService};

pub struct KafkaNotifyUpdateProductService {
    pub producer: FutureProducer,
    pub topic_name: String,
}

#[async_trait]
impl NotifyUpdateProductService for KafkaNotifyUpdateProductService {

    async fn notify_product_update(&self, product_id: &String) {
        self.producer.send(
            FutureRecord::to(&self.topic_name)
                .key(&product_id)
                .payload(&CommandType::UPDATE.to_string()),
            Timeout::After(Duration::from_millis(0)),
        ).await.expect("Could not send update command to Kafka");
    }

    async fn notify_product_delete(&self, product_id: &String) {
        self.producer.send(
            FutureRecord::to(&self.topic_name)
                .key(&product_id)
                .payload(&CommandType::DELETE.to_string()),
            Timeout::After(Duration::from_millis(0)),
        ).await.expect("Could not send delete command to Kafka");
    }
}