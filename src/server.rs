#[macro_use]
extern crate rbatis;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use diesel::connection;
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use log::{error, info, warn, LevelFilter};
use rbatis::core::value::DateTimeNow;
use rbatis::crud::CRUD;
use rbatis::rbatis::Rbatis;
use simple_logger::SimpleLogger;
use tokio_postgres::NoTls;
use tonic::{transport::Server, Request, Response, Status};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub struct MyGreeter {
    db_connect: Rbatis,
}

#[crud_enable]
#[derive(Clone, Debug)]
pub struct GrpcMessages {
    pub id: Option<u32>,
    pub message: Option<String>,
    pub client_address: Option<String>,
    //pub received_at_db: Option<chrono::NaiveDateTime>,
    pub received_at_server: Option<String>,
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
        db_connect: MyGreeter::make_db_connection(
            config
                .as_str(),
        )
        .await
        .unwrap(),
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

        self.write_to_postgres(grpc_message, grpc_client, chrono::NaiveDateTime::now())
            .await
            .unwrap();

        Ok(Response::new(reply))
    }
}

#[async_trait]
trait DbWork {
    async fn make_db_connection(db_url: &str) -> Result<rbatis::rbatis::Rbatis, rbatis::Error>;
}
#[async_trait]
impl DbWork for MyGreeter {
    async fn make_db_connection(db_url: &str) -> Result<Rbatis, rbatis::Error> {
        let rb = Rbatis::new();
        rb.link(db_url).await.map_err(|e| return e);
        Ok(rb)
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
    fn new(db_connect: Rbatis) -> MyGreeter {
        MyGreeter { db_connect }
    }

    async fn write_to_postgres(
        &self,
        grpc_message: &str,
        grpc_client: &str,
        datetime: chrono::NaiveDateTime, // TODO: return the record with resulting id
    ) -> Result<(u64), rbatis::Error> {
        let count_chars_to_reduce_datetime_str = 26;
        let datetime = datetime.to_string();
        //let datetime = &datetime.to_rfc3339().replace("T", " ")[..count_chars_to_reduce_datetime_str];
        let message = GrpcMessages {
            id: None,
            message: Some(grpc_message.to_string()),
            client_address: Some(grpc_client.to_string()),
            //received_at_db: None,
            received_at_server: Some(datetime),
        };

        match self.db_connect
            .save("", &message)
            .await {
            Ok(val)=>val,
            Err(e) => return Err(e),
        };

        Ok(0)
    }
}

fn last_day_in_next_month(date_now: chrono::NaiveDate) -> u32 {
    let mut date;
    if date_now.month() == 12 {
        date = chrono::NaiveDate::from_ymd(date_now.year() + 1, 1, 1);
    } else {
        date = chrono::NaiveDate::from_ymd(date_now.year(), date_now.month() + 2, 1);
    }
    let date = date + chronoutil::RelativeDuration::days(-1);
    date.day()
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_last_day_in_next_month_normal_1() {
        let date = chrono::NaiveDate::from_ymd(2021, 5, 20);
        assert_eq!(last_day_in_next_month(date), 30);
    }
    #[test]
    fn test_last_day_in_next_month_normal_2() {
        let date = chrono::NaiveDate::from_ymd(2021, 1, 2);
        assert_eq!(last_day_in_next_month(date), 28);
    }
    #[test]
    fn test_last_day_in_next_month_next_year() {
        let date = chrono::NaiveDate::from_ymd(2020, 12, 2);
        assert_eq!(last_day_in_next_month(date), 31);
    }
    #[test]
    fn test_last_day_in_next_month_leap_year() {
        let date = chrono::NaiveDate::from_ymd(2020, 1, 2);
        assert_eq!(last_day_in_next_month(date), 29);
    }
    #[test]
    fn test_last_day_in_next_month_minus_year() {
        let date = chrono::NaiveDate::from_ymd(-9, 5, 2);
        assert_eq!(last_day_in_next_month(date), 30);
    }
}
