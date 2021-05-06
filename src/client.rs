use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use simple_logger::SimpleLogger;

use log::info;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::from_env().init().unwrap();

    let address = std::env::var("SERVER_ADDRESS");

    let address = match address {
        Ok(addr) => addr,
        Err(error) => panic!("Problem reading the env variable: {:?}", error),
    };

    let mut client = GreeterClient::connect(format!("http://{}", address)).await?;

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    info!("RESPONSE={:?}", response);

    Ok(())
}
