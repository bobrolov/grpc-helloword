use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use log::{error, info, warn, LevelFilter};
use simple_logger::SimpleLogger;
use tokio_postgres::NoTls;
use tonic::{transport::Server, Request, Response, Status};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub struct MyGreeter {
    db_client: tokio_postgres::Client,
    db_table: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let address = match std::env::var("SERVER_ADDRESS")
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

    let config = MyGreeter::read_string_from_env("POSTGRES_CONFIG").unwrap();

    let greeter = MyGreeter::new(
        MyGreeter::make_db_client(config.as_str()).await.unwrap(),
        MyGreeter::read_string_from_env("POSTGRES_TABLE")
            .unwrap()
            .to_string(),
    );

    Server::builder()
        .add_service(GreeterServer::new(greeter))
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

        self.write_to_postgres(grpc_message, grpc_client, Utc::now())
            .await
            .unwrap();

        Ok(Response::new(reply))
    }
}

trait New {
    fn new(db_client: tokio_postgres::Client, db_table: String) -> Self;
}
impl New for MyGreeter {
    fn new(db_client: tokio_postgres::Client, db_table: String) -> Self {
        MyGreeter {
            db_client,
            db_table,
        }
    }
}

#[async_trait]
trait DbWork {
    async fn make_db_client(config: &str) -> Result<tokio_postgres::Client, tokio_postgres::Error>;
    fn read_string_from_env(env_key: &str) -> Result<String, std::env::VarError>;
}
#[async_trait]
impl DbWork for MyGreeter {
    async fn make_db_client(config: &str) -> Result<tokio_postgres::Client, tokio_postgres::Error> {
        let (client, connection) = match tokio_postgres::connect(config, NoTls).await {
            Ok((cli, conn)) => (cli, conn),
            Err(err) => {
                error!("Can't connect to postgres db");
                return Err(err);
            }
        };
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        Ok(client)
    }
    fn read_string_from_env(env_key: &str) -> Result<String, std::env::VarError> {
        match std::env::var(env_key) {
            Ok(val) => {
                info!("env_key: {}, found: {}", env_key, val);
                Ok(val)
            }
            Err(err) => {
                error!("Problem with read table_name from env");
                Err(err)
            }
        }
    }
}

impl MyGreeter {
    // TODO: use DI and read about it (additionally sometimes it's smart to use traits not implementations hi SOLID)

    async fn write_to_postgres(
        &self,
        grpc_message: &str,
        grpc_client: &str,
        datetime: DateTime<Utc>,
        // TODO: return the record with resulting id
    ) -> Result<(u64), tokio_postgres::Error> {
        let count_chars_to_reduce_datetime_str = 26;
        let datetime =
            &datetime.to_rfc3339().replace("T", " ")[..count_chars_to_reduce_datetime_str];

        // TODO: read about ORMs and diesel in particular
        // Answer the question: what pain do they solve and why are needed
        let db_query = format!(
            "INSERT INTO {} (message, client_address, received_at_server) VALUES ($1, $2, $3)",
            self.db_table
        )
        .as_str();
        let result = self
            .db_client
            .execute(
                &self.db_client.prepare(db_query).await.unwrap(),
                &[&grpc_message, &grpc_client, &datetime],
            )
            .await
            .unwrap();

        // let id_row = match &self
        //     .db_client
        //     .execute("INSERT INTO grpc_messages (message, client_address, received_at_server) VALUES ($1, $2, $3)", &[&grpc_message, &grpc_client, &datetime])
        //     .await
        // {
        //     Ok(insert_row) => insert_row,
        //     Err(e) => panic!("{:?}", e),
        // };

        Ok(result)
    }
}
