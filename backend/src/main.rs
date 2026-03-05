use crate::{errors::ServerError, load_balancer::LoadBalancer};


mod load_balancer;
mod server;
mod errors;
mod jsons;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    
    match run_server().await{
        Ok(_) => {},
        Err(err) => {
            println!("{}", err)
        }
    }
}


async fn run_server() -> Result<(), ServerError>{
    let lb = LoadBalancer::new().await;
    

    Ok(())
}