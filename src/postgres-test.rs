use postgres::{Client, NoTls};
use simple_logger::SimpleLogger;
use log::info;

fn main() {

    SimpleLogger::new().init().unwrap();

    let db_address = std::env::var("POSTGRES_ADDRESS");
    let db_address = match db_address {
        Ok(addr) => addr,
        Err(error) => panic!("Problem reading the db_address from env: {:?}", error),
    };
    let db_port = std::env::var("POSTGRES_PORT");
    let db_port = match db_port {
        Ok(port) => port,
        Err(error) => panic!("Problem reading the db_port from env: {:?}", error),
    };
    let db_user = std::env::var("POSTGRES_USER");
    let db_user = match db_user {
        Ok(user) => user,
        Err(error) => panic!("Problem reading the db user from env: {:?}", error),
    };
    let db_password = std::env::var("POSTGRES_PASSWORD");
    let db_password = match db_password {
        Ok(pass) => pass,
        Err(error) => panic!("Problem reading the db_password from env: {:?}", error),
    };
    let db_table = std::env::var("POSTGRES_TABLE");
    let db_table = match db_table {
        Ok(table) => table,
        Err(error) => panic!("Problem reading the db_table from env: {:?}", error),
    };

    let config_connection = format!("host={} user={} password={}", db_address, db_user,db_password);
    info!("{}",config_connection);

    let client = Client::connect(config_connection.as_str(), NoTls);
    let mut client = match client {
        Ok(client) => client,
        Err(error) => panic!("Problem with connection to postgres: {:?}", error),
    };

    let par1 = "test_prog";
    let par2 = "test_prog2";
    //let query = format!("INSERT INTO {} (message, client) VALUES ($1, $2)", db_table);

    let insert_row =  client.execute("INSERT INTO grpc_messages (message, client) VALUES ($1, $2)",
                   &[&par1, &par2]);
    let insert_row = match insert_row {
        Ok(insert_row) => insert_row,
        Err(error) => panic!("Problem with insert command: {:?}", error),
    };

    info!("Results of insert operation: {}", insert_row);

}