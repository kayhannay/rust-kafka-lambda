# Example Rust Kafka DynamoDB Lambda

This ia a small example AWS Lambda application that reads data from a Kafka topic, stores them in DynamoDB and 
then sends a command on another Kafka topic.

## Run tests

There are a few integration tests that use localstack. So you can run and modify these tests to play around a bit.

To run the testsyou can use make:

```
make test
```

## Build Lambda ZIP file

To build a ZIP file that can be used with AWS Lambda, a Docker container is used. There is a make command to handle
all this:

```
make build
```

After that you can find the ZIP file at target/bootstrap.zip.

## Deployment

This Lambda expects a self managed Kafka and a DynamoDB table. It is using an Amazon Linux 2 runtime. Set the 
following environment variables (see main.rs for details):

* PRODUCT_TABLE_NAME
* UPDATE_CONVERTED_PRODUCT_TOPIC_NAME
* KAFKA_USERNAME
* KAFKA_PASSWORD
* KAFKA_BOOTSTRAP_SERVERS
