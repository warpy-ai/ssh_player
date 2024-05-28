use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use log::info;
use russh::{server::*, Channel, ChannelId};
use russh_keys::key::PublicKey;
use std::*;

use bubblers::{cli_builder, config::CliConfig};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut server = AppServer::new();

    server.run().await.expect("failed to run server");
}

#[derive(Clone)]
struct AppServer {
    clients: Arc<Mutex<HashMap<usize, App>>>,
    id: usize,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![russh_keys::key::KeyPair::generate_ed25519().unwrap()],
            ..Default::default()
        };
        self.run_on_address(Arc::new(config), ("127.0.0.1", 2222))
            .await?;

        Ok(())
    }
}

impl Server for AppServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait::async_trait]
impl Handler for AppServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let mut clients = self.clients.lock().unwrap();
        let app = App::new();

        clients.insert(self.id, app);

        // add the input form
        let input = show_input_form();
        session.data(channel.id(), input.into());

        info!("new session: {}", self.id);
        let response = format!("id: {}", self.id);

        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        match data {
            b"exit" => {
                self.clients.lock().unwrap().remove(&self.id);
                session.close(channel)
            }
            _ => {}
        }

        Ok(())
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }
}

pub fn show_input_form() -> String {
    let mut input_form = CliConfig::new("Input form", "0.1", "SImple input form");

    input_form.add_input(
        "input_form",
        "Name of the user",
        "Your name",
        "",
        "Your Name",
    );

    let result = cli_builder::execute_cli(&input_form);

    // let name = result.get("input_form").unwrap_or(&"Unknown".to_string());
    "Hello".to_string()
}

struct App {
    pub counter: usize,
}

impl App {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}
