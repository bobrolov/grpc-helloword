use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use simple_logger::SimpleLogger;
use tokio_postgres::{Error, NoTls};

use log::{info, warn};

#[macro_use]
extern crate derive_new;

use chrono::Utc;
use postgres::tls::NoTlsStream;
use postgres::Socket;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(new)]
pub struct MyGreeter {
    db_client: tokio_postgres::Client,
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Got a request from {:?}", request.remote_addr());

        let grpc_client = &request.remote_addr().unwrap().to_string();
        let msg = format!("Not hello {}!", request.into_inner().name);
        let grpc_message = &msg.clone();

        let mut reply = hello_world::HelloReply { message: msg };

        &self
            .write_to_postgres(grpc_message, grpc_client)
            .await
            .unwrap();
        /*
        let res = match write_to_postgres(grpc_message, grpc_client).await {
            Ok(res) => res,
            Err(error) => panic!("Error: {}", error),
        };
        */
        Ok(Response::new(reply))
    }
}
impl MyGreeter {
    async fn write_to_postgres(&self, grpc_message: &str, grpc_client: &str) -> Result<(), Error> {
        let db_table = match std::env::var("POSTGRES_TABLE") {
            Ok(table) => table,
            Err(error) => panic!("Problem reading the db_table from env: {:?}", error),
        };

        let datetime = Utc::now().to_rfc3339().replace("T", " ");
        let datetime = &datetime[..26];

        let db_statement = &self.db_client
            .prepare(
                format!(
                    "INSERT INTO {} (message, client_address, received_at_server) VALUES ($1, $2, $3)",
                    db_table
                )
                    .as_str(),
            )
            .await
            .unwrap();
        let insert_row = &self
            .db_client
            .execute(db_statement, &[&grpc_message, &grpc_client, &datetime])
            .await;
        let insert_row = match insert_row {
            Ok(insert_row) => insert_row,
            Err(error) => panic!("Problem with insert command: {:?}", error),
        };
        info!("Results of insert operation: {}", insert_row);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::from_env().init().unwrap();

    let address = match std::env::var("ADDRESS")
        .unwrap_or_else(|err| {
            warn!("Can't read env, set to default address");
            "0.0.0.0:4000".to_string()
        })
        .parse()
    {
        Ok(address) => {
            info!("Server will be listening on {:?}", address);
            address
        }
        Err(error) => panic!("Problem with parsing this address: {:?}", error),
    };

    let greeter = MyGreeter::new(connect_to_postgres().await.unwrap());
    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(address)
        .await?;

    Ok(())
}

async fn connect_to_postgres() -> Result<tokio_postgres::Client, Error> {
    let config_connection = match std::env::var("POSTGRES_CONFIG") {
        Ok(addr) => addr,
        Err(error) => panic!("Problem reading the config from env: {:?}", error),
    };
    info!("Read config from env: {}", config_connection);

    let (client, connection) = tokio_postgres::connect(config_connection.as_str(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            panic!("Postgres connection error: {}", err);
        }
    });

    Ok(client)
}
