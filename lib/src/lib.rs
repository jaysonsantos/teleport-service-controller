use eyre::{Result, WrapErr};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Service;
use kube::api::{ListParams, WatchEvent};
use kube::{Api, Client};
use kube_runtime::watcher::{self, Event};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tracing::{error, info, instrument};

use crate::batcher::{Batcher, Command};

mod batcher;
mod teleport_config;

const LABEL_SELECTOR: &str = "teleport/enabled=true";

pub struct Controller {
    client: Client,
    commands_sender: UnboundedSender<Command>,
    batcher_handler: JoinHandle<Result<()>>,
}

impl Controller {
    pub async fn new() -> Result<Self> {
        let client = Client::try_default()
            .await
            .wrap_err("failed to get a kube client")?;
        let (sender, receiver) = unbounded_channel::<Command>();
        let handler = tokio::spawn(Batcher::new(&client, Duration::from_secs(3), receiver).run());
        Ok(Self {
            client,
            commands_sender: sender,
            batcher_handler: handler,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let api: Api<Service> = Api::all(self.client.clone());
        let mut stream = watcher(api, 
                // &ListParams::default().timeout(10).labels(LABEL_SELECTOR),
                ListParams::default()
            ).boxed();
        while let Some(event) = stream.try_next().await? {
            match event {
                Event::Applied(event)
                | Event::Deleted(event)
                | Event::Restarted(event) => self.configure(&event).await?,
                Event::Bookmark(event) => {
                    info!(message = "Ignoring bookmark event", ?event.types, ?event.metadata.resource_version);
                }
                Event::Error(event) => {
                    error!(message = "Watcher error", ?event)
                }
            }
        }

        info!("No more events, flushing any existing services.");
        self.commands_sender.send(Command::Close)?;
        // self.batcher_handler.await?;
        Ok(())
    }

    #[instrument(
        skip(self, service),
        fields(
            service_name = service.metadata.name.as_deref().unwrap_or(""),
            service_namespace = service.metadata.namespace.as_deref().unwrap_or(""),
        )
    )]
    async fn configure(&self, service: &Service) -> Result<()> {
        info!("Enqueueing change");
        self.commands_sender
            .send(Command::Enqueue(service.clone()))
            .wrap_err("failed to enqueue command")?;
        Ok(())
    }
}
