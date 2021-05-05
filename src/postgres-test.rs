//use postgres::{Client, NoTls};
use tokio_postgres::{NoTls, Error};
use simple_logger::SimpleLogger;
use log::{info};


#[tokio::main]
async fn main() -> Result<(), Error> {

    SimpleLogger::new().init().unwrap();

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

    let par1 = "test_tokio_attempt";
    let par2 = "test_tokio_attempt2";

    let db_statement = client.prepare(format!("INSERT INTO {} (message, client) VALUES ($1, $2)", db_table).as_str()).await.unwrap();
    let insert_row =  client.execute(&db_statement,
                   &[&par1, &par2]).await;
    let insert_row = match insert_row {
        Ok(insert_row) => insert_row,
        Err(error) => panic!("Problem with insert command: {:?}", error),
    };
    info!("Results of insert operation: {}", insert_row);

    Ok(())
}