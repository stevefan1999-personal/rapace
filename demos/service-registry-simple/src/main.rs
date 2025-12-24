use rapace::{BufferPool, RpcSession, prelude::*};
use rapace_cell::DispatcherBuilder;
use std::sync::Arc;

#[rapace::service]
pub trait Calculator {
    async fn add(&self, a: i32, b: i32) -> Option<i32>;
}

#[derive(Debug, Clone, thiserror::Error, rapace::facet::Facet)]
#[facet(crate = rapace::facet)]
#[repr(u8)]
pub enum CalculatorError {
    #[error("integer overflow")]
    Overflow,
}

#[rapace::service]
pub trait CalculatorV2 {
    async fn add(&self, a: i32, b: i32) -> Result<i32, CalculatorError>;
}

// Implement your service...
#[derive(Clone, Debug)]
struct MyCalculator;

impl Calculator for MyCalculator {
    // V1 of the method returns Option<i32>
    async fn add(&self, a: i32, b: i32) -> Option<i32> {
        a.checked_add(b)
    }
}

impl CalculatorV2 for MyCalculator {
    // V2 of the method returns Result<i32, CalculatorError>
    async fn add(&self, a: i32, b: i32) -> Result<i32, CalculatorError> {
        a.checked_add(b).ok_or(CalculatorError::Overflow)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (client, server) = Transport::stream_pair();
    let (client, server) = (
        Arc::new(RpcSession::new(client)),
        Arc::new(RpcSession::new(server)),
    );

    let server = tokio::spawn({
        server.set_dispatcher(
            DispatcherBuilder::new()
                .add_service(CalculatorServer::new(MyCalculator))
                .add_service(CalculatorV2Server::new(MyCalculator))
                .build(BufferPool::new()),
        );

        server.run()
    });

    let client = tokio::spawn(async move {
        tokio::spawn(client.clone().run());

        {
            let client = ServiceRegistry::with_global(|registry| {
                CalculatorRegistryClient::new(client.clone(), &registry)
            });
            println!("{:?}", client.add(1, 2).await.unwrap());
        }
        {
            let client = ServiceRegistry::with_global(|registry| {
                CalculatorV2RegistryClient::new(client.clone(), &registry)
            });
            println!("{:?}", client.add(1, 2).await.unwrap());
        }

        Ok::<(), anyhow::Error>(())
    });

    tokio::select! {
        res = server => { res??; },
        res = client => { res??; },
    }

    Ok(())
}
