use eyre::Result;
use k8s_openapi::api::core::v1::Service;
use kube::Client;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{interval, Duration, Interval};
use tracing::{info, instrument};

pub struct Batcher {
    client: Client,
    services: Vec<Service>,
    timer: Interval,
    receiver: UnboundedReceiver<Command>,
}

#[derive(Debug)]
pub enum Command {
    Enqueue(Service),
    Close,
}

impl Batcher {
    pub fn new(
        client: &Client,
        batching_time: Duration,
        receiver: UnboundedReceiver<Command>,
    ) -> Self {
        Self {
            client: client.clone(),
            timer: interval(batching_time),
            services: vec![],
            receiver,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            select! {
                _ = self.timer.tick() => {
                    if self.should_flush() {
                        self.flush().await?;
                    }
                }
                command = self.receiver.recv() => {
                    if let Some(command) = command {
                        match command {
                            Command::Enqueue(service) => self.push_service(service).await,
                            Command::Close => break,
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        if self.should_flush() {
            self.flush().await?;
        }
        Ok(())
    }

    pub async fn push_service(&mut self, service: Service) {
        self.services.push(service.clone());
    }

    fn should_flush(&self) -> bool {
        !self.services.is_empty()
    }

    #[instrument(skip(self))]
    async fn flush(&mut self) -> Result<()> {
        info!(
            messaging = "Configuring teleport",
            services_number = self.services.len()
        );
        self.services.clear();
        Ok(())
    }
}
