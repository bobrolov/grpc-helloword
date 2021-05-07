use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
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

    let greeter = MyGreeter {
        db_client: MyGreeter::make_db_client(config.as_str()).await.unwrap(),
        db_table: MyGreeter::read_string_from_env("POSTGRES_TABLE")
            .unwrap()
            .to_string(),
    };

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

#[async_trait]
trait DbWork {
    async fn make_db_client(config: &str) -> Result<tokio_postgres::Client, tokio_postgres::Error>;
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
}

trait EnvWork {
    fn read_string_from_env(env_key: &str) -> Result<String, std::env::VarError>;
}
impl EnvWork for MyGreeter {
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

    fn new(&self, db_client: tokio_postgres::Client, db_table: String) -> MyGreeter {
        MyGreeter {
            db_client,
            db_table,
        }
    }

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

        let db_query = "INSERT INTO grpc_messages (message, client_address, received_at_server) VALUES ($1, $2, $3)";
        // let db_query = format!(
        //     "INSERT INTO {} (message, client_address, received_at_server) VALUES ($1, $2, $3)",
        //     self.db_table
        // )
        // .as_str();

        let result = match self
            .db_client
            .execute(
                &self.db_client.prepare(db_query).await.unwrap(),
                &[&grpc_message, &grpc_client, &datetime],
            )
            .await
        {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        Ok(result)
    }
}

fn last_day_in_next_month(date_now: chrono::Date<Utc>) -> i32 {
    let year = date_now.year();
    let next_month = {
        if date_now.month() == 12 { 1 } else { date_now.month() + 1 }
    };

    let is_leap_year = year % 4 == 0 && year % 100 != 0;

    match next_month {
        1 | 3 | 5 | 7 | 8 | 10 | 13 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year { 29 } else { 28 }
        },
        _ => 0,
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_last_day_in_next_month_normal_1() {
        let date = Utc.ymd(2021, 5, 2);
        assert_eq!(last_day_in_next_month(date), 30);
    }
    #[test]
    fn test_last_day_in_next_month_normal_2() {
        let date = Utc.ymd(2021, 1, 2);
        assert_eq!(last_day_in_next_month(date), 28);
    }
    #[test]
    fn test_last_day_in_next_month_next_year() {
        let date = Utc.ymd(2020, 12, 2);
        assert_eq!(last_day_in_next_month(date), 31);
    }
    #[test]
    fn test_last_day_in_next_month_leap_year() {
        let date = Utc.ymd(2020, 1, 2);
        assert_eq!(last_day_in_next_month(date), 29);
    }
    #[test]
    fn test_last_day_in_next_month_minus_year() {
        let date = Utc.ymd(-9, 5, 2);
        assert_eq!(last_day_in_next_month(date), 30);
    }
}
