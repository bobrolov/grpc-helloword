#[macro_use]
extern crate diesel;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use diesel::table;

fn main() {
    println!("123");
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = "postgresql://postgres:password@localhost";
    PgConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}


table! {
    grpc_messages(id) {
        id -> Serial,
        message -> Text,
        client_address -> Text,
        received_at_db -> Timestamp,
        received_at_server -> Text,
    }
}