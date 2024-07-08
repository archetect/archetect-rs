use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

use crate::errors::ArchetectError;
use crate::proto::archetect_service_server::ArchetectServiceServer as ArchetectServiceGrpcServer;
use crate::server::ArchetectServiceCore;

#[derive(Clone)]
pub struct ArchetectServer {
    core: ArchetectServiceCore,
    service_port: u16,
    listener: Arc<Mutex<Option<TcpListener>>>,
}

pub struct ArchetectServerBuilder {
    core: ArchetectServiceCore,
}

impl ArchetectServerBuilder {
    pub fn new(core: ArchetectServiceCore) -> ArchetectServerBuilder {
        ArchetectServerBuilder { core }
    }

    pub async fn build(self) -> Result<ArchetectServer, ArchetectError> {
        let configuration = self.core.prototype().configuration();
        let listener = TcpListener::bind((configuration.server().host(), configuration.server().port()))
            .await
            .map_err(|err| ArchetectError::IoError(err))?;
        let addr = listener.local_addr()?;

        Ok(ArchetectServer {
            core: self.core,
            service_port: addr.port(),
            listener: Arc::new(Mutex::new(Some(listener))),
        })
    }
}

impl ArchetectServer {
    pub fn builder(core: ArchetectServiceCore) -> ArchetectServerBuilder {
        ArchetectServerBuilder::new(core)
    }

    pub fn service_port(&self) -> u16 {
        self.service_port
    }

    pub async fn serve(&self) -> Result<(), ArchetectError> {
        let listener = self.listener.lock().await.take().expect("Listener Expected");

        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<ArchetectServiceGrpcServer<ArchetectServiceCore>>()
            .await;

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(crate::proto::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        let server = Server::builder()
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(ArchetectServiceGrpcServer::new(self.core.clone()));

        tracing::info!("Archetect Server started on {}", listener.local_addr()?);

        // TODO: Create a proper Error for server stuff
        server
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .map_err(|err| ArchetectError::GeneralError(err.to_string()))?;

        Ok(())
    }
}
