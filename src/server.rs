use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use simple_logger::SimpleLogger;
use tokio_postgres::{Error, NoTls};

use log::{info, warn};

use chrono::{DateTime, Utc};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

//#[derive(new)]
pub struct MyGreeter {
    db_client: tokio_postgres::Client,
    db_table: String,
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

    Server::builder()
        .add_service(GreeterServer::new(MyGreeter::new().await))
        .serve(address)
        .await?;

    Ok(())
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
        let reply = hello_world::HelloReply { message: msg };

        &self
            .write_to_postgres(grpc_message, grpc_client, Utc::now())
            .await
            .unwrap();
        Ok(Response::new(reply))
    }
}
impl MyGreeter {
    async fn new() -> MyGreeter {
        let config_connection = match std::env::var("POSTGRES_CONFIG") {
            Ok(addr) => addr,
            Err(error) => panic!("Problem reading the config from env: {:?}", error),
        };
        info!("Read postgres config from env: {}", config_connection);

        let (client, connection) = tokio_postgres::connect(config_connection.as_str(), NoTls)
            .await
            .unwrap();

        tokio::spawn(async move {
            if let Err(err) = connection.await {
                panic!("Postgres connection error: {}", err);
            }
        });

        let table_name = std::env::var("POSTGRES_TABLE").unwrap_or_default();

        MyGreeter {
            db_client: client,
            db_table: table_name,
        }
    }
    async fn write_to_postgres(
        &self,
        grpc_message: &str,
        grpc_client: &str,
        datetime: DateTime<Utc>,
    ) -> Result<(), Error> {
        let datetime = &datetime.to_rfc3339().replace("T", " ")[..26];
        //let datetime = &datetime[..26];

        let db_statement = &self.db_client
            .prepare(
                format!(
                    "INSERT INTO {} (message, client_address, received_at_server) VALUES ($1, $2, $3)",
                    &self.db_table
                ).as_str(),
            ).await.unwrap();

        match &self
            .db_client
            .execute(db_statement, &[&grpc_message, &grpc_client, &datetime])
            .await
        {
            Ok(insert_row) => info!("Results of insert operation: {}", insert_row),
            Err(error) => panic!("Problem with insert command: {:?}", error),
        };

        Ok(())
    }
}
