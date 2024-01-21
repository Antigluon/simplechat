use anyhow::anyhow;
use anyhow::Result;
use axum::extract::ConnectInfo;
use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use axum::response::IntoResponse;
use axum::routing;
use axum::Router;
use futures::Sink;
use futures::SinkExt;
use futures::StreamExt;
use lib::sync::funnel_in;
use lib::sync::sink;
use lib::{
    command::{Command, CommandRegistry},
    user::{
        GuestUser,
        User::{self, Guest},
    },
};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{collections::HashSet, ops::Deref};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

mod commands;

struct ServerState {
    tx: broadcast::Sender<String>,
    taken_usernames: Mutex<HashSet<String>>,
    commands: RwLock<CommandRegistry>,
}

impl ServerState {
    async fn parse_command<'a>(
        &self,
        message: &'a str,
    ) -> Result<(impl Deref<Target = Command> + '_, &'a str)> {
        let (name, args) = message
            .split_once(char::is_whitespace)
            .unwrap_or((message, ""));
        let guard = self.commands.read().await;
        let guard = RwLockReadGuard::try_map(guard, |registry| {
            let command = registry.get(name);
            command
        })
        .map_err(|_| anyhow!("Command `/{}` not found.", name))?;
        Ok((guard, args))
    }

    fn free_username(&self, username: &str) {
        let mut all_names = self
            .taken_usernames
            .lock()
            .expect("Failed to acquire name list");
        all_names.remove(username);
    }

    fn broadcast(&self, message: &str) -> Result<()> {
        self.tx.send(message.to_string())?;
        Ok(())
    }

    fn subscribe_listener<S>(&self, send_handle: S) -> JoinHandle<()>
    where
        S: Sink<Message> + Send + Unpin + 'static,
        S::Error: Debug,
    {
        let broadcast_reciever = tokio_stream::wrappers::BroadcastStream::new(self.tx.subscribe());
        tokio::spawn(async move {
            // while let Ok(message) = rx.recv().await {
            //     // tracing::trace!("(broadcast) {message}");
            //     let result = send_handle.send(Message::Text(message)).await;
            //     if result.is_err() {
            //         // client disconnected
            //         tracing::info!("failed to send: {result:?}");
            //         break;
            //     }
            // }
            let _ = broadcast_reciever
                .map(|msg| {
                    let msg = msg.unwrap();
                    tracing::trace!("(broadcast) {msg}");
                    Ok(Message::Text(msg))
                })
                .forward(send_handle)
                .await;
        })
    }
}

async fn establish_connection(
    socket: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    tracing::info!("Client connecting from {}", addr);
    socket.on_upgrade(|socket| register_client(socket, state))
}

async fn register_guest(state: &ServerState, raw_username: &str) -> Result<User> {
    let mut all_names = state
        .taken_usernames
        .lock()
        .expect("Failed to lock mutex when registering client");
    let user = Guest(GuestUser::new(&all_names, raw_username)?);
    all_names.insert(user.username().to_string());
    tracing::info!("Registered guest `{}`", user.username());
    Ok(user)
}

async fn register_client(stream: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut reciever) = stream.split();
    let mut user: Option<User> = None;
    // Consume text messages from the client.
    while let Some(Ok(Message::Text(message))) = reciever.next().await {
        match register_guest(&state, &message).await {
            Ok(guest) => {
                user = Some(guest);
                break;
            }
            Err(e) => sender.send(Message::Text(format!("{}", e))).await.unwrap(),
        }
        continue;
    }

    let user =
        user.expect("Failed to register client: Connection broke before username was accepted.");

    // Subscribe to broadcast messages.
    // This is done before the join message is sent to ensure that the client receives the message.
    let whisper_handle = funnel_in(sender);
    let mut send_task = state.subscribe_listener(sink(whisper_handle.clone()));
    let mut whisper_handle = sink(whisper_handle);

    let saved_username = user.username().to_string();
    let saved_state = state.clone();

    let _ = whisper_handle
        .send(Message::Text(format!("Welcome, {}!", user.username())))
        .await
        .map(|_| tracing::info!("Sent welcome message to {}", user.username()))
        .map_err(|e| tracing::error!("Failed to send welcome message: {e:?}"));

    state
        .broadcast(&format!("{} has joined.", user.username()))
        .unwrap();

    // Consume text messages from the client.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(message))) = reciever.next().await {
            let message = message.trim();
            if message.is_empty() {
                continue;
            }
            if message.starts_with('/') {
                // Handle commands.
                tracing::trace!("(command) {message}");
                let invocation = state.parse_command(&message[1..]).await;
                match invocation {
                    Ok((command, args)) => {
                        tracing::trace!("(invocation) {} {args}", command.name);
                        match command.execute(&user, args) {
                            Ok(response) => whisper_to(&mut whisper_handle, &response).await,
                            Err(e) => whisper_to(&mut whisper_handle, &format!("{}", e)).await,
                        }
                    }
                    Err(e) => {
                        tracing::trace!("(failed parse): {e}");
                        whisper_to(&mut whisper_handle, &format!("{}", e)).await
                    }
                }
            } else {
                // Broadcast message.
                let message = format!("{}: {}", user.username(), message);
                tracing::trace!("(message) {message}");
                state.broadcast(&message).unwrap();
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let state = saved_state;
    let leave_msg = format!("(leave) {} has left.", saved_username);
    tracing::info!("{}", leave_msg);
    let _ = state.broadcast(&leave_msg);
    state.free_username(&saved_username);
}

async fn whisper_to(sender: &mut (impl SinkExt<Message> + Unpin), message: &str) {
    let _ = sender.send(Message::Text(message.to_string())).await;
}

const IP: &str = "127.0.0.1";
const PORT: u16 = 1234;
const CAPACITY: usize = 65536;
const LOGGING_FILTER: &str = "server=info,axum=trace,tokio=info,hyper=trace";

#[tokio::main]
async fn main() {
    // Initialize logging with tracing_subscriber
    let _log_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| LOGGING_FILTER.into());
    tracing_subscriber::registry()
        .with(_log_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, _rx) = broadcast::channel(CAPACITY);
    let server_state = Arc::new(ServerState {
        tx,
        taken_usernames: Mutex::new(HashSet::new()),
        commands: RwLock::new(commands::default_commands()),
    });
    let app = Router::new()
        .route("/connect", routing::get(establish_connection))
        .with_state(server_state);
    let app = app.into_make_service_with_connect_info::<SocketAddr>();

    let listener = TcpListener::bind(format!("{IP}:{PORT}"))
        .await
        .expect("Failed to bind to port {PORT}");

    tracing::info!(
        "Listening on {}",
        listener
            .local_addr()
            .expect("Listener could not bind to local address.")
    );

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
