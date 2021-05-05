

use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use simple_logger::SimpleLogger;
use tokio_postgres::{NoTls, Error};

use log::info;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Got a request from {:?}", request.remote_addr());

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        let grpc_message = "54321";
        let grpc_client = "12345";
        let res = match write_to_postgres(grpc_message,grpc_client).await {
            Ok(res) => res,
            Err(error) => panic!("Error: {}", error),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    SimpleLogger::new().init().unwrap();

    let address = match std::env::var("ADDRESS") {
        Ok(addr) => addr,
        Err(error) => panic!("Problem reading the env variable: {:?}", error),
    };

    let addr = address.parse().unwrap();
    let greeter = MyGreeter::default();


    info!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}

async fn write_to_postgres(grpc_message: &str, grpc_client: &str) -> Result<(), Error> {


    let config_connection = match std::env::var("POSTGRES_CONFIG") {
        Ok(addr) => addr,
        Err(error) => panic!("Problem reading the config from env: {:?}", error),
    };
    info!("Read config from env: {}",config_connection);

    let db_table = match std::env::var("POSTGRES_TABLE") {
        Ok(table) => table,
        Err(error) => panic!("Problem reading the db_table from env: {:?}", error),
    };

    let (client, connection) = tokio_postgres::connect(config_connection.as_str(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            panic!("Postgres connection error: {}", err);
        }
    });

    let db_statement = client.prepare(format!("INSERT INTO {} (message, client) VALUES ($1, $2)", db_table).as_str()).await.unwrap();
    let insert_row =  client.execute(&db_statement,
                                     &[&grpc_message, &grpc_client]).await;
    let insert_row = match insert_row {
        Ok(insert_row) => insert_row,
        Err(error) => panic!("Problem with insert command: {:?}", error),
    };
    info!("Results of insert operation: {}", insert_row);

    Ok(())
}