use aws_config::BehaviorVersion;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use aws_config::meta::region::RegionProviderChain;
use aws_lambda_events::encodings::MillisecondTimestamp;
use aws_lambda_events::event::kafka::{KafkaEvent, KafkaRecord};
use aws_sdk_dynamodb::config::Credentials;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
    ScalarAttributeType,
};
use aws_sdk_dynamodb::Client;
use chrono::DateTime;
use futures::StreamExt;
use http::HeaderMap;
use lambda_runtime::tracing::level_filters::LevelFilter;
use lambda_runtime::tracing::subscriber::EnvFilter;
use lambda_runtime::tracing::Level;
use lambda_runtime::{Config, Context, LambdaEvent};
use lib_base64::Base64;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::FutureProducer;
use rdkafka::{ClientConfig, Message};
use rust_kafka_lambda::adapter::dynamodb_store_converted_product_service::DynamoDbStoreConvertedProductService;
use rust_kafka_lambda::adapter::kafka_notify_update_product_service::KafkaNotifyUpdateProductService;
use rust_kafka_lambda::business::save_converted_product_use_case::SaveConvertedProductUseCase;
use rust_kafka_lambda::domain::product::Product;
use rust_kafka_lambda::handler::lambda_kafka_event_handler::LambdaKafkaEventHandler;
use testcontainers::runners::AsyncRunner;

mod kafka;
mod localstack;
#[tokio::test]
async fn happy_path() {
    init_logging();
    let localstack = localstack::Localstack::default().start().await;
    let localstack_port = localstack.get_host_port_ipv4(4566).await;
    let kafka_node = kafka::Kafka::default().start().await;
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT).await
    );
    println!("Bootstrap servers: {}", bootstrap_servers);

    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;
    let check_client = client.clone();

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        save_converted_product_use_case: SaveConvertedProductUseCase {
            store_converted_product_service: DynamoDbStoreConvertedProductService {
                dynamo_db_client: client,
                table_name: dynamodb_table_name.to_string(),
            },
            notify_update_product_service: KafkaNotifyUpdateProductService {
                producer,
                topic_name: topic.to_string(),
            },
        },
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler
        .handle_lambda_kafka_event(lambda_event)
        .await
        .expect("TODO: panic message");

    // then

    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client
        .get_item()
        .table_name(dynamodb_table_name)
        .key("id", AttributeValue::S("p1".to_string()))
        .send()
        .await
        .expect("DB error");
    assert_eq!(
        "p1",
        item_output
            .item()
            .expect("Product not found in database!")
            .get("id")
            .unwrap()
            .as_s()
            .unwrap()
    );
    //assert_eq!("{}", item_output.item().unwrap().get("data").unwrap().as_s().unwrap());
    println!(
        "Converted product: {}",
        item_output
            .item()
            .unwrap()
            .get("data")
            .unwrap()
            .as_s()
            .unwrap()
    );

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["UPDATE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    }
}

#[tokio::test]
async fn delete_product_tombstone() {
    init_logging();
    let localstack = localstack::Localstack::default().start().await;
    let localstack_port = localstack.get_host_port_ipv4(4566).await;
    let kafka_node = kafka::Kafka::default().start().await;
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT).await
    );

    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;
    let check_client = client.clone();

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        save_converted_product_use_case: SaveConvertedProductUseCase {
            store_converted_product_service: DynamoDbStoreConvertedProductService {
                dynamo_db_client: client,
                table_name: dynamodb_table_name.to_string(),
            },
            notify_update_product_service: KafkaNotifyUpdateProductService {
                producer,
                topic_name: topic.to_string(),
            },
        },
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler
        .handle_lambda_kafka_event(lambda_event)
        .await
        .expect("TODO: panic message");

    // then
    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client
        .get_item()
        .table_name(dynamodb_table_name)
        .key("id", AttributeValue::S("p1".to_string()))
        .send()
        .await
        .expect("f");
    assert_eq!(
        "p1",
        item_output
            .item()
            .expect("Product not found in database!")
            .get("id")
            .unwrap()
            .as_s()
            .unwrap()
    );

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["UPDATE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    }

    println!("Prepare tombstone");
    let lambda_event = create_lambda_event(topic, "p1", &None);

    // when
    println!("Call lambda handler");
    lambda_event_handler
        .handle_lambda_kafka_event(lambda_event)
        .await
        .expect("TODO: panic message");

    // then
    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client
        .get_item()
        .table_name(dynamodb_table_name)
        .key("id", AttributeValue::S("p1".to_string()))
        .send()
        .await
        .expect("f");
    assert_eq!(None, item_output.item());

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["DELETE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    }
}

#[tokio::test]
async fn change_detection_save() {
    init_logging();
    let localstack = localstack::Localstack::default().start().await;
    let localstack_port = localstack.get_host_port_ipv4(4566).await;
    let kafka_node = kafka::Kafka::default().start().await;
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT).await
    );

    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        save_converted_product_use_case: SaveConvertedProductUseCase {
            store_converted_product_service: DynamoDbStoreConvertedProductService {
                dynamo_db_client: client,
                table_name: dynamodb_table_name.to_string(),
            },
            notify_update_product_service: KafkaNotifyUpdateProductService {
                producer,
                topic_name: topic.to_string(),
            },
        },
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler
        .handle_lambda_kafka_event(lambda_event.clone())
        .await
        .expect("TODO: panic message");
    lambda_event_handler
        .handle_lambda_kafka_event(lambda_event)
        .await
        .expect("TODO: panic message");

    // then
    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), message_stream.next())
            .await
            .is_ok()
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(5), message_stream.next())
            .await
            .is_err()
    );
}

fn initialize_kafka(bootstrap_servers: String, topic: &str) -> (FutureProducer, StreamConsumer) {
    //## Kafka
    println!("Initialize Kafka");

    let producer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer>()
        .expect("Failed to create Kafka FutureProducer");

    let consumer = ClientConfig::new()
        .set("group.id", "testcontainer-rs")
        .set("bootstrap.servers", &bootstrap_servers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create::<StreamConsumer>()
        .expect("Failed to create Kafka StreamConsumer");

    consumer
        .subscribe(&[topic])
        .expect("Failed to subscribe to a topic");

    (producer, consumer)
}

async fn initialize_dynamodb(localstack_port: u16, dynamodb_table_name: &str) -> Client {
    //## localstack
    println!("Initialize Localstack and DynamoDB");

    let region_provider = RegionProviderChain::default_provider().or_else("eu-central-1");
    let shared_config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(Credentials::new(
            "example", "example", None, None, "example",
        ))
        .load()
        .await;
    let mut dynamodb_client_builder = aws_sdk_dynamodb::config::Builder::from(&shared_config);
    dynamodb_client_builder =
        dynamodb_client_builder.endpoint_url(format!("http://127.0.0.1:{}/", localstack_port));
    let dynamodb_client = Client::from_conf(dynamodb_client_builder.build());
    let key_name = "id";
    let ks = KeySchemaElement::builder()
        .attribute_name(key_name)
        .key_type(KeyType::Hash)
        .build()
        .expect("Could not create KeySchemaElement");
    let ad = AttributeDefinition::builder()
        .attribute_name(key_name)
        .attribute_type(ScalarAttributeType::S)
        .build()
        .expect("Could not create AttributeDefinition");
    let pt = ProvisionedThroughput::builder()
        .read_capacity_units(10)
        .write_capacity_units(5)
        .build()
        .expect("Could not create ProvisionedThroughput");
    dynamodb_client
        .create_table()
        .table_name(dynamodb_table_name)
        .key_schema(ks)
        .attribute_definitions(ad)
        .provisioned_throughput(pt)
        .send()
        .await
        .expect("Could not create table");

    dynamodb_client
}

fn create_lambda_event_from_file(
    topic: &str,
    product_id: &str,
    file_name: &str,
) -> LambdaEvent<KafkaEvent> {
    let file = fs::File::open(file_name).expect("file should open read only");
    let product: Product = serde_json::from_reader(file).expect("file should be proper JSON");
    create_lambda_event(topic, product_id, &Some(product))
}

fn create_lambda_event(
    topic: &str,
    product_id: &str,
    product_data: &Option<Product>,
) -> LambdaEvent<KafkaEvent> {
    let mut product = None;
    let mut record_header: HashMap<String, Vec<i8>> = HashMap::new();
    record_header.insert(
        String::from("sampleHeader"),
        String::from("sampleHeaderValue")
            .into_bytes()
            .into_iter()
            .map(|c| c as i8)
            .collect::<_>(),
    );
    if product_data.is_some() {
        product = Some(
            serde_json::to_string(product_data.as_ref().unwrap())
                .unwrap()
                .encode()
                .unwrap(),
        );
    }
    let kafka_record = KafkaRecord {
        topic: Some(topic.to_string()),
        partition: 0,
        offset: 0,
        timestamp: MillisecondTimestamp(DateTime::default()),
        timestamp_type: None,
        key: Some(product_id.to_string().encode().unwrap()),
        value: product,
        headers: vec![record_header],
    };
    let kafka_event = KafkaEvent {
        event_source: None,
        event_source_arn: None,
        records: HashMap::from([(topic.to_string(), vec![kafka_record])]),
        bootstrap_servers: None,
    };
    let config = Arc::new(Config::default());
    let mut context_header = HeaderMap::new();
    context_header.insert("lambda-runtime-deadline-ms", "1000".parse().unwrap());
    let context = Context::new("some_req", config, &context_header).unwrap();
    LambdaEvent {
        payload: kafka_event,
        context,
    }
}

fn init_logging() {
    let log_format = env::var("AWS_LAMBDA_LOG_FORMAT").unwrap_or_default();
    let log_level = Level::from_str(&env::var("AWS_LAMBDA_LOG_LEVEL").unwrap_or_default())
        .unwrap_or(Level::INFO);

    let collector = tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::from_level(log_level).into())
                .from_env_lossy(),
        );

    if log_format.eq_ignore_ascii_case("json") {
        collector.json().try_init().unwrap_or(())
    } else {
        collector.try_init().unwrap_or(())
    }
}
